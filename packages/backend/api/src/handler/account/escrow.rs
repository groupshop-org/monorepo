use base64::Engine;
use getrandom::fill as random_fill;
use groupshop_backend_shared::prelude::*;

use crate::{
    config::ESCROW_WALLET_PROOF_LIFETIME_MILLIS,
    db::{
        account::participation::UserParticipationDb,
        product::{batch::ProductBatchDb, catalog::ProductCatalogDb},
    },
    prelude::*,
    solana::{
        authority_pubkey_from_config, authority_sign_deposit_transaction,
        authority_sign_self_refund_transaction, build_deposit_recipe, build_self_refund_recipe,
        decode_blockhash, deposit_message, derive_participation_pda, ensure_pool_initialized,
        product_amount_base_units, rpc_get_account_data, rpc_get_latest_blockhash,
        rpc_send_transaction, self_refund_message, transaction_bytes, verify_wallet_signature,
        ParticipationAccount, Pubkey, SIGNATURE_PLACEHOLDER,
    },
    utils::{req_to_json, sign_bytes, verify_bytes},
};

pub async fn handle_escrow_deposit_intent(
    ctx: &mut ApiContext,
    req: HttpRequest,
) -> ApiResult<AccountEscrowDepositIntentResponse> {
    let req: AccountEscrowDepositIntentRequest = req_to_json(req).await?;
    let wallet = normalize_wallet_address(&req.wallet_address)?;
    let uid = ctx.unchecked_uid().clone();
    let product = ProductCatalogDb::load_by_id(ctx, &req.product_id).await?;

    if !product.is_active {
        return Err(ApiError::Validation(
            "product is not currently accepting deposits".to_string(),
        ));
    }

    if req.quantity == 0 {
        return Err(ApiError::Validation(
            "quantity must be at least 1".to_string(),
        ));
    }

    // The buyer picks their own quantity. `product_amount_base_units`
    // multiplies price by quantity (not MOQ) — MOQ is the *batch-wide*
    // threshold, no longer the per-buyer commitment.
    let product_amount = product_amount_base_units(product.price_cents, req.quantity)?;
    let shipping_amount = 0u64;
    let issued_at_ms = js_sys::Date::now() as u64;
    let expires_at_ms = issued_at_ms + ESCROW_WALLET_PROOF_LIFETIME_MILLIS;
    let authority_address = authority_pubkey_from_config(&ctx.config.solana)?.to_string();

    // Resolve the batch this deposit will target. We DO NOT create the
    // D1 batch row here — the build endpoint will, once the wallet
    // signature is verified — but we do bind the proof to a specific
    // batch_id so it can't be replayed against a future batch.
    let batch_id = match ProductBatchDb::load_active(ctx, &req.product_id).await? {
        Some(row) if row.pipeline_status == "open" => row.batch_id,
        Some(row) => row
            .batch_id
            .checked_add(1)
            .ok_or_else(|| ApiError::Validation("batch id overflow".to_string()))?,
        None => 0,
    };

    let claims = EscrowWalletProofClaims {
        uid: uid.clone(),
        product_id: req.product_id.clone(),
        product_name: product.name.clone(),
        wallet_address: wallet.clone(),
        network: ctx.config.solana.network.clone(),
        rpc_url: ctx.config.solana.rpc_url_client.clone(),
        program_id: ctx.config.solana.market_program_id.clone(),
        usdc_mint: ctx.config.solana.usdc_mint.clone(),
        authority_address: authority_address.clone(),
        product_amount_base_units: product_amount,
        shipping_amount_base_units: shipping_amount,
        issued_at_ms,
        expires_at_ms,
        nonce: random_nonce()?,
        batch_id,
        quantity: req.quantity,
    };
    let proof_token = sign_proof_token(ctx, claims).await?;
    let challenge_message = challenge_message(&proof_token.claims);
    let proof_token = proof_token.encode_str()?;
    let total_amount = product_amount
        .checked_add(shipping_amount)
        .ok_or_else(|| ApiError::Validation("deposit total overflow".to_string()))?;

    Ok(AccountEscrowDepositIntentResponse {
        network: ctx.config.solana.network.clone(),
        rpc_url: ctx.config.solana.rpc_url_client.clone(),
        program_id: ctx.config.solana.market_program_id.clone(),
        usdc_mint: ctx.config.solana.usdc_mint.clone(),
        authority_address,
        wallet_address: wallet,
        user_id: uid,
        product_amount_base_units: product_amount,
        shipping_amount_base_units: shipping_amount,
        total_amount_base_units: total_amount,
        token_decimals: 6,
        product_name: product.name,
        challenge_message,
        challenge_expires_at_ms: expires_at_ms,
        proof_token,
        batch_id,
        quantity: req.quantity,
    })
}

pub async fn handle_escrow_deposit_build(
    ctx: &mut ApiContext,
    req: HttpRequest,
) -> ApiResult<AccountEscrowDepositBuildResponse> {
    let req: AccountEscrowDepositBuildRequest = req_to_json(req).await?;
    let proof_token = EscrowWalletProofToken::decode_str(&req.proof_token)?;
    verify_proof_token(ctx, &proof_token).await?;

    let claims = proof_token.claims;
    let now = js_sys::Date::now() as u64;
    if now > claims.expires_at_ms {
        return Err(ApiError::Validation(
            "wallet proof has expired; request a new challenge".to_string(),
        ));
    }

    if claims.uid != *ctx.unchecked_uid() {
        return Err(ApiError::Validation(
            "wallet proof does not belong to the current user".to_string(),
        ));
    }

    let wallet = Pubkey::from_base58(&claims.wallet_address)?;
    verify_wallet_signature(
        &wallet,
        challenge_message(&claims).as_bytes(),
        &req.wallet_signature_base64,
    )?;

    // Look up the catalog so we can pin the on-chain `threshold` to the
    // current MOQ. The threshold is baked into the Pool at init time and
    // doesn't change per deposit.
    let product = ProductCatalogDb::load_by_id(ctx, &claims.product_id).await?;
    if !product.is_active {
        return Err(ApiError::Validation(
            "product is not currently accepting deposits".to_string(),
        ));
    }

    // Make sure the batch row exists in D1 before initializing the pool
    // on-chain. The on-chain init is idempotent (`ensure_pool_initialized`
    // skips if the account already exists) and the D1 row is too
    // (`insert_if_absent`).
    ProductBatchDb::insert_if_absent(
        ctx,
        &claims.product_id,
        claims.batch_id,
        product.minimum_order_quantity,
    )
    .await?;

    ensure_pool_initialized(
        ctx,
        &claims.product_id,
        claims.batch_id,
        product.minimum_order_quantity,
    )
    .await?;

    let recent_blockhash = rpc_get_latest_blockhash(ctx).await?;
    let blockhash_bytes = decode_blockhash(&recent_blockhash)?;
    let recipe = build_deposit_recipe(
        &ctx.config.solana,
        &claims.product_id,
        &wallet,
        &claims.uid,
        claims.product_amount_base_units,
        claims.shipping_amount_base_units,
        claims.batch_id,
        claims.quantity as u64,
    )?;
    let message = deposit_message(&recipe, blockhash_bytes);

    // The deposit message places the buyer at signer index 0 (fee payer) and
    // the authority at signer index 1. Both slots are zero-filled here so
    // Phantom signs first; the backend adds the authority signature only after
    // the signed message returns through `escrow-deposit-submit`.
    let buyer_signature_placeholder = SIGNATURE_PLACEHOLDER.to_vec();
    let authority_signature_placeholder = SIGNATURE_PLACEHOLDER.to_vec();
    let tx_bytes = transaction_bytes(
        &[buyer_signature_placeholder, authority_signature_placeholder],
        &message,
    );
    let transaction_base64 = base64::engine::general_purpose::STANDARD.encode(&tx_bytes);

    let total_amount = claims
        .product_amount_base_units
        .checked_add(claims.shipping_amount_base_units)
        .ok_or_else(|| ApiError::Validation("deposit total overflow".to_string()))?;

    Ok(AccountEscrowDepositBuildResponse {
        network: claims.network,
        rpc_url: claims.rpc_url,
        program_id: claims.program_id,
        usdc_mint: claims.usdc_mint,
        authority_address: claims.authority_address,
        wallet_address: claims.wallet_address,
        recent_blockhash,
        product_amount_base_units: claims.product_amount_base_units,
        shipping_amount_base_units: claims.shipping_amount_base_units,
        total_amount_base_units: total_amount,
        product_name: claims.product_name,
        user_id: claims.uid,
        buyer_associated_token_account: recipe.buyer_ata.to_string(),
        pool_address: recipe.pool.to_string(),
        vault_address: recipe.vault.to_string(),
        participation_address: recipe.participation.to_string(),
        participation_bump: recipe.participation_bump,
        transaction_base64,
        batch_id: claims.batch_id,
    })
}

pub async fn handle_escrow_deposit_submit(
    ctx: &mut ApiContext,
    req: HttpRequest,
) -> ApiResult<AccountEscrowTransactionSubmitResponse> {
    let req: AccountEscrowDepositSubmitRequest = req_to_json(req).await?;
    let proof_token = EscrowWalletProofToken::decode_str(&req.proof_token)?;
    verify_proof_token(ctx, &proof_token).await?;

    let claims = proof_token.claims;
    let now = js_sys::Date::now() as u64;
    if now > claims.expires_at_ms {
        return Err(ApiError::Validation(
            "wallet proof has expired; request a new challenge".to_string(),
        ));
    }
    if claims.uid != *ctx.unchecked_uid() {
        return Err(ApiError::Validation(
            "wallet proof does not belong to the current user".to_string(),
        ));
    }

    let wallet = Pubkey::from_base58(&claims.wallet_address)?;
    let recipe = build_deposit_recipe(
        &ctx.config.solana,
        &claims.product_id,
        &wallet,
        &claims.uid,
        claims.product_amount_base_units,
        claims.shipping_amount_base_units,
        claims.batch_id,
        claims.quantity as u64,
    )?;
    let signed_tx_bytes = base64::engine::general_purpose::STANDARD
        .decode(&req.signed_transaction_base64)
        .map_err(|err| ApiError::Base64Decode(err.to_string()))?;
    let fully_signed_tx =
        authority_sign_deposit_transaction(&ctx.config.solana, &signed_tx_bytes, &recipe)?;
    let tx_signature = rpc_send_transaction(ctx, &fully_signed_tx).await?;

    Ok(AccountEscrowTransactionSubmitResponse { tx_signature })
}

/// Records a successfully submitted deposit into D1. The client calls this
/// after Phantom returns a signature; the server independently verifies the
/// deposit by reading the on-chain Participation PDA — never trust client
/// claims about what landed on-chain. The function is idempotent: replaying
/// the same `tx_signature` just refreshes the D1 row with the latest
/// on-chain snapshot.
pub async fn handle_escrow_deposit_confirm(
    ctx: &mut ApiContext,
    req: HttpRequest,
) -> ApiResult<AccountEscrowDepositConfirmResponse> {
    let req: AccountEscrowDepositConfirmRequest = req_to_json(req).await?;
    let uid = ctx.unchecked_uid().clone();
    let product = ProductCatalogDb::load_by_id(ctx, &req.product_id).await?;

    // Resolve the active batch on the server side. We don't take batch_id
    // from the client here — the proof_token already bound the deposit
    // to a batch; this handler just looks it up.
    let active_batch = ProductBatchDb::load_active(ctx, &req.product_id)
        .await?
        .ok_or_else(|| {
            ApiError::Validation(
                "no active batch for product — was the deposit-build flow skipped?".to_string(),
            )
        })?;
    let batch_id = active_batch.batch_id;

    let (participation_pda, _bump) =
        derive_participation_pda(&ctx.config.solana, &req.product_id, &uid, batch_id)?;

    let bytes = rpc_get_account_data(ctx, &participation_pda)
        .await?
        .ok_or_else(|| {
            ApiError::Validation(
                "on-chain participation account not found yet — try again in a moment".to_string(),
            )
        })?;
    let participation = ParticipationAccount::from_bytes(&bytes)?;

    // Defense in depth: the PDA seed binding already ensures `user_id` ==
    // `uid`, but verify the field anyway in case the program ever changes
    // how it derives Participation accounts.
    if participation.user_id != uid.inner() {
        return Err(ApiError::Validation(
            "on-chain participation user_id does not match signed-in user".to_string(),
        ));
    }

    let wallet_address = participation.wallet.to_base58();
    UserParticipationDb::upsert(
        ctx,
        &uid,
        &req.product_id,
        batch_id,
        &wallet_address,
        participation.product_amount,
        participation.shipping_amount,
        participation.quantity,
        &req.tx_signature,
    )
    .await?;

    let committed_units =
        UserParticipationDb::units_for_batch(ctx, &req.product_id, batch_id).await?;

    Ok(AccountEscrowDepositConfirmResponse {
        product_id: req.product_id,
        product_amount_base_units: participation.product_amount,
        shipping_amount_base_units: participation.shipping_amount,
        committed_units,
        minimum_order_quantity: product.minimum_order_quantity,
        batch_id,
    })
}

/// First leg of the buyer-initiated refund. Mirrors the deposit-build
/// shape: builds the on-chain SelfRefund transaction with empty signer
/// slots. Phantom signs the buyer slot first; the backend adds the authority
/// signature and submits in the follow-up endpoint.
pub async fn handle_escrow_refund_build(
    ctx: &mut ApiContext,
    req: HttpRequest,
) -> ApiResult<AccountEscrowRefundBuildResponse> {
    let req: AccountEscrowRefundBuildRequest = req_to_json(req).await?;
    let uid = ctx.unchecked_uid().clone();
    let wallet = Pubkey::from_base58(&req.wallet_address)?;

    // Confirm the participation we'd be refunding actually belongs to
    // this signed-in user. The on-chain instruction also checks but we
    // fail fast here for a better error message.
    let (participation_pda, _bump) =
        derive_participation_pda(&ctx.config.solana, &req.product_id, &uid, req.batch_id)?;
    let bytes = rpc_get_account_data(ctx, &participation_pda)
        .await?
        .ok_or_else(|| ApiError::Validation("no participation found for this batch".to_string()))?;
    let participation = ParticipationAccount::from_bytes(&bytes)?;
    if participation.refunded {
        return Err(ApiError::Validation(
            "this participation has already been refunded".to_string(),
        ));
    }
    if participation.user_id != uid.inner() {
        return Err(ApiError::Validation(
            "on-chain participation user_id does not match signed-in user".to_string(),
        ));
    }
    if participation.wallet.to_base58() != req.wallet_address {
        return Err(ApiError::Validation(
            "wallet address does not match the on-chain participation".to_string(),
        ));
    }

    let recipe = build_self_refund_recipe(
        &ctx.config.solana,
        &req.product_id,
        &wallet,
        &uid,
        req.batch_id,
    )?;
    let recent_blockhash = rpc_get_latest_blockhash(ctx).await?;
    let blockhash_bytes = decode_blockhash(&recent_blockhash)?;
    let message = self_refund_message(&recipe, blockhash_bytes);
    let buyer_signature_placeholder = SIGNATURE_PLACEHOLDER.to_vec();
    let authority_signature_placeholder = SIGNATURE_PLACEHOLDER.to_vec();
    let tx_bytes = transaction_bytes(
        &[buyer_signature_placeholder, authority_signature_placeholder],
        &message,
    );
    let transaction_base64 = base64::engine::general_purpose::STANDARD.encode(&tx_bytes);

    let total = participation
        .product_amount
        .checked_add(participation.shipping_amount)
        .ok_or_else(|| ApiError::Validation("refund total overflow".to_string()))?;

    Ok(AccountEscrowRefundBuildResponse {
        network: ctx.config.solana.network.clone(),
        rpc_url: ctx.config.solana.rpc_url_client.clone(),
        program_id: ctx.config.solana.market_program_id.clone(),
        usdc_mint: ctx.config.solana.usdc_mint.clone(),
        authority_address: recipe.authority.to_base58(),
        wallet_address: req.wallet_address,
        batch_id: req.batch_id,
        recent_blockhash,
        buyer_associated_token_account: recipe.buyer_ata.to_base58(),
        product_amount_base_units: participation.product_amount,
        shipping_amount_base_units: participation.shipping_amount,
        total_amount_base_units: total,
        transaction_base64,
    })
}

pub async fn handle_escrow_refund_submit(
    ctx: &mut ApiContext,
    req: HttpRequest,
) -> ApiResult<AccountEscrowTransactionSubmitResponse> {
    let req: AccountEscrowRefundSubmitRequest = req_to_json(req).await?;
    let uid = ctx.unchecked_uid().clone();
    let wallet = Pubkey::from_base58(&req.wallet_address)?;

    let (participation_pda, _bump) =
        derive_participation_pda(&ctx.config.solana, &req.product_id, &uid, req.batch_id)?;
    let bytes = rpc_get_account_data(ctx, &participation_pda)
        .await?
        .ok_or_else(|| ApiError::Validation("no participation found for this batch".to_string()))?;
    let participation = ParticipationAccount::from_bytes(&bytes)?;
    if participation.refunded {
        return Err(ApiError::Validation(
            "this participation has already been refunded".to_string(),
        ));
    }
    if participation.user_id != uid.inner() {
        return Err(ApiError::Validation(
            "on-chain participation user_id does not match signed-in user".to_string(),
        ));
    }
    if participation.wallet.to_base58() != req.wallet_address {
        return Err(ApiError::Validation(
            "wallet address does not match the on-chain participation".to_string(),
        ));
    }

    let recipe = build_self_refund_recipe(
        &ctx.config.solana,
        &req.product_id,
        &wallet,
        &uid,
        req.batch_id,
    )?;
    let signed_tx_bytes = base64::engine::general_purpose::STANDARD
        .decode(&req.signed_transaction_base64)
        .map_err(|err| ApiError::Base64Decode(err.to_string()))?;
    let fully_signed_tx =
        authority_sign_self_refund_transaction(&ctx.config.solana, &signed_tx_bytes, &recipe)?;
    let tx_signature = rpc_send_transaction(ctx, &fully_signed_tx).await?;

    Ok(AccountEscrowTransactionSubmitResponse { tx_signature })
}

/// Second leg of the buyer-initiated refund. Reads the on-chain
/// participation to confirm the SelfRefund instruction landed (refunded
/// flag flipped) and then mirrors that into D1.
pub async fn handle_escrow_refund_confirm(
    ctx: &mut ApiContext,
    req: HttpRequest,
) -> ApiResult<AccountEscrowRefundConfirmResponse> {
    let req: AccountEscrowRefundConfirmRequest = req_to_json(req).await?;
    let uid = ctx.unchecked_uid().clone();
    let product = ProductCatalogDb::load_by_id(ctx, &req.product_id).await?;

    let (participation_pda, _bump) =
        derive_participation_pda(&ctx.config.solana, &req.product_id, &uid, req.batch_id)?;
    let bytes = rpc_get_account_data(ctx, &participation_pda)
        .await?
        .ok_or_else(|| {
            ApiError::Validation(
                "on-chain participation account missing — has the refund landed?".to_string(),
            )
        })?;
    let participation = ParticipationAccount::from_bytes(&bytes)?;
    if !participation.refunded {
        return Err(ApiError::Validation(
            "on-chain participation hasn't been marked refunded yet".to_string(),
        ));
    }
    if participation.user_id != uid.inner() {
        return Err(ApiError::Validation(
            "on-chain participation user_id does not match signed-in user".to_string(),
        ));
    }

    UserParticipationDb::mark_refunded(ctx, &uid, &req.product_id, req.batch_id, &req.tx_signature)
        .await?;

    let committed_units =
        UserParticipationDb::units_for_batch(ctx, &req.product_id, req.batch_id).await?;

    Ok(AccountEscrowRefundConfirmResponse {
        product_id: req.product_id,
        batch_id: req.batch_id,
        committed_units,
        minimum_order_quantity: product.minimum_order_quantity,
    })
}

async fn sign_proof_token(
    ctx: &ApiContext,
    claims: EscrowWalletProofClaims,
) -> ApiResult<EscrowWalletProofToken> {
    let signature = AuthTokenSignature::from(
        sign_bytes(&ctx.config.token_signing_key, claims.encode()?).await?,
    );
    Ok(EscrowWalletProofToken { claims, signature })
}

async fn verify_proof_token(ctx: &ApiContext, token: &EscrowWalletProofToken) -> ApiResult<()> {
    let valid = verify_bytes(
        &ctx.config.token_signing_key,
        token.claims.encode()?,
        token.signature.as_ref(),
    )
    .await?;

    if valid {
        Ok(())
    } else {
        Err(ApiError::Validation(
            "wallet proof token signature is invalid".to_string(),
        ))
    }
}

fn challenge_message(claims: &EscrowWalletProofClaims) -> String {
    format!(
        "groupshop.org wants you to authorize an escrow deposit with your Solana account:\n{}\n\nAuthorize Groupshop to co-sign the escrow deposit transaction for this product.\n\nUser: {}\nProduct: {}\nBatch: {}\nQuantity: {}\nProduct Amount (USDC-6): {}\nShipping Amount (USDC-6): {}\nCluster: {}\nNonce: {}\nIssued At (ms): {}\nExpiration Time (ms): {}",
        claims.wallet_address,
        claims.uid,
        claims.product_id,
        claims.batch_id,
        claims.quantity,
        claims.product_amount_base_units,
        claims.shipping_amount_base_units,
        claims.network,
        claims.nonce,
        claims.issued_at_ms,
        claims.expires_at_ms,
    )
}

fn normalize_wallet_address(value: &str) -> ApiResult<String> {
    Ok(Pubkey::from_base58(value.trim())?.to_string())
}

fn random_nonce() -> ApiResult<String> {
    let mut bytes = [0u8; 16];
    random_fill(&mut bytes)
        .map_err(|err| ApiError::Crypto(format!("failed to create nonce: {err}")))?;
    Ok(base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes))
}
