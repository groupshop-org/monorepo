use gloo_net::http::{Request, Response};
use groupshop_backend_shared::prelude::*;
use http::{Method, StatusCode};
use serde::{de::DeserializeOwned, Serialize};

use crate::{
    constants::AUTH_SESSION_TOKEN_STORAGE_KEY,
    error::{FrontendError, FrontendResult},
    util::storage::{
        delete_local_storage, get_local_storage, has_local_storage, set_local_storage,
    },
};

pub struct ApiClient {
    endpoint: &'static str,
}

impl ApiClient {
    pub fn new(endpoint: &'static str) -> Self {
        Self { endpoint }
    }

    async fn api_response<T: ApiRouteResponse>(&self) -> FrontendResult<T::Res> {
        self.request_json::<(), T::Res>(T::METHOD, T::ROUTE, None::<&()>, false)
            .await
    }

    async fn api_request_response<T: ApiRouteRequestResponse>(
        &self,
        req: &T::Req,
    ) -> FrontendResult<T::Res> {
        self.request_json::<T::Req, T::Res>(T::METHOD, T::ROUTE, Some(req), false)
            .await
    }

    async fn api_empty<T: ApiRouteEmpty>(&self) -> FrontendResult<()> {
        self.request_text::<()>(T::METHOD, T::ROUTE, None::<&()>, false)
            .await
            .map(|_| ())
    }

    async fn api_request<T: ApiRouteRequest>(&self, req: &T::Req) -> FrontendResult<()> {
        self.request_text::<T::Req>(T::METHOD, T::ROUTE, Some(req), false)
            .await
            .map(|_| ())
    }

    async fn request_json<Req: Serialize, Res: DeserializeOwned>(
        &self,
        method: Method,
        route: ApiRoute,
        body: Option<&Req>,
        recursed: bool,
    ) -> FrontendResult<Res> {
        let text = self.request_text(method, route, body, recursed).await?;
        serde_json::from_str::<Res>(&text).map_err(|err| FrontendError::ParseResponse { err, text })
    }

    async fn request_text<Req: Serialize>(
        &self,
        method: Method,
        route: ApiRoute,
        body: Option<&Req>,
        recursed: bool,
    ) -> FrontendResult<String> {
        let response = self.request_raw(method, route, body, recursed).await?;
        let status = response.status();
        let text = response.text().await?;

        if !(200..300).contains(&status) {
            if status == StatusCode::UNAUTHORIZED.as_u16() {
                let _ = delete_local_storage(AUTH_SESSION_TOKEN_STORAGE_KEY);
            }
            let api_err = serde_json::from_str::<ApiError>(&text)
                .unwrap_or(ApiError::HttpStatus { status, body: text });
            return Err(api_err.into());
        }

        Ok(text)
    }

    async fn request_raw<Req: Serialize>(
        &self,
        method: Method,
        route: ApiRoute,
        body: Option<&Req>,
        recursed: bool,
    ) -> FrontendResult<Response> {
        let url = format!("{}/{}", self.endpoint.trim_end_matches('/'), route);

        let mut builder = match method {
            Method::GET => Request::get(&url),
            Method::POST => Request::post(&url),
            Method::PUT => Request::put(&url),
            Method::DELETE => Request::delete(&url),
            _ => return Err(FrontendError::UnsupportedHttpMethod(method)),
        }
        .credentials(web_sys::RequestCredentials::Include);

        let try_refresh = if !recursed {
            matches!(route.auth_requirement(), Some(AuthRequirement::Session)
                if !has_local_storage(AUTH_SESSION_TOKEN_STORAGE_KEY)?)
        } else {
            false
        };

        if try_refresh {
            let _ = Box::pin(self.request_text::<()>(
                AuthRefreshRoute::METHOD,
                AuthRefreshRoute::ROUTE,
                None::<&()>,
                true,
            ))
            .await?;
        }

        if matches!(route.auth_requirement(), Some(AuthRequirement::Session)) {
            if let Some(session_token) = get_local_storage(AUTH_SESSION_TOKEN_STORAGE_KEY)? {
                builder = builder.header("Authorization", &format!("Bearer {session_token}"));
            }
        }

        builder = builder.header("Accept", "application/json");

        let response = match body {
            Some(body) => {
                let json = serde_json::to_string(body)
                    .map_err(|err| FrontendError::ParseRequest { err })?;
                builder
                    .header("Content-Type", "application/json")
                    .body(json)?
                    .send()
                    .await?
            }
            None => builder.send().await?,
        };

        if let Some(token) = response.headers().get(HEADER_AUTH_SESSION_TOKEN_UPDATE) {
            let _ = AuthToken::decode_str(&token)?;
            set_local_storage(AUTH_SESSION_TOKEN_STORAGE_KEY, &token)?;
        }

        Ok(response)
    }

    pub async fn auth_refresh(&self) -> FrontendResult<()> {
        self.api_empty::<AuthRefreshRoute>().await
    }

    pub async fn auth_signout(&self) -> FrontendResult<()> {
        self.api_empty::<AuthSignoutRoute>().await
    }

    pub async fn auth_open_id_signin(
        &self,
        provider: OpenIdProvider,
    ) -> FrontendResult<AuthOpenIdConnectResponse> {
        self.api_request_response::<AuthOpenIdConnectRoute>(&AuthOpenIdConnectRequest { provider })
            .await
    }

    pub async fn auth_open_id_finalize_query(
        &self,
        token: AuthToken,
    ) -> FrontendResult<AuthOpenIdFinalizeQueryResponse> {
        self.api_request_response::<AuthOpenIdFinalizeQueryRoute>(&AuthOpenIdFinalizeQueryRequest {
            token,
        })
        .await
    }

    pub async fn auth_open_id_finalize_exec(
        &self,
        token: AuthToken,
        consent: Option<AuthRegistrationConsent>,
    ) -> FrontendResult<()> {
        self.api_request::<AuthOpenIdFinalizeExecRoute>(&AuthOpenIdFinalizeExecRequest {
            token,
            consent,
        })
        .await
    }

    pub async fn auth_email_send_verify(&self) -> FrontendResult<()> {
        self.api_empty::<AuthEmailAddressSendVerificationRoute>()
            .await
    }

    pub async fn auth_email_confirm(&self, token: AuthToken) -> FrontendResult<()> {
        self.api_request::<AuthEmailAddressConfirmVerificationRoute>(
            &AuthEmailAddressConfirmVerificationRequest { token },
        )
        .await
    }

    pub async fn auth_email_password_register(
        &self,
        email: String,
        password: String,
        consent: AuthRegistrationConsent,
    ) -> FrontendResult<()> {
        self.api_request::<AuthEmailPasswordRegisterRoute>(&AuthEmailPasswordRegisterRequest {
            email,
            password,
            consent,
        })
        .await
    }

    pub async fn auth_email_password_signin(
        &self,
        email: String,
        password: String,
    ) -> FrontendResult<()> {
        self.api_request::<AuthEmailPasswordSigninRoute>(&AuthEmailPasswordSigninRequest {
            email,
            password,
        })
        .await
    }

    pub async fn auth_email_password_send_reset(
        &self,
        email: Option<String>,
    ) -> FrontendResult<()> {
        match email {
            Some(email) => {
                self.api_request::<AuthEmailPasswordSendResetAnyRoute>(
                    &AuthEmailPasswordSendResetAnyRequest { email },
                )
                .await
            }
            None => self.api_empty::<AuthEmailPasswordSendResetMeRoute>().await,
        }
    }

    pub async fn auth_email_password_confirm_reset(
        &self,
        token: AuthToken,
        new_password: String,
    ) -> FrontendResult<()> {
        self.api_request::<AuthEmailPasswordConfirmResetRoute>(
            &AuthEmailPasswordConfirmResetRequest {
                token,
                new_password,
            },
        )
        .await
    }

    pub async fn account_profile(&self) -> FrontendResult<AccountProfile> {
        self.api_response::<AccountProfileRoute>().await
    }

    pub async fn account_profile_update(
        &self,
        req: &AccountProfileUpdateRequest,
    ) -> FrontendResult<()> {
        self.api_request::<AccountProfileUpdateRoute>(req).await
    }

    pub async fn account_username_check(
        &self,
        username: String,
    ) -> FrontendResult<AccountUsernameCheckResponse> {
        self.api_request_response::<AccountUsernameCheckRoute>(&AccountUsernameCheckRequest {
            username,
        })
        .await
    }

    pub async fn account_username_update(
        &self,
        username: String,
    ) -> FrontendResult<AccountUsernameUpdateResponse> {
        self.api_request_response::<AccountUsernameUpdateRoute>(&AccountUsernameUpdateRequest {
            username,
        })
        .await
    }

    pub async fn admin_list_users(
        &self,
        req: &AdminListUsersRequest,
    ) -> FrontendResult<AdminListUsersResponse> {
        self.api_request_response::<AdminListUsersRoute>(req).await
    }

    pub async fn admin_update_user(
        &self,
        req: &AdminUpdateUserRequest,
    ) -> FrontendResult<AdminUpdateUserResponse> {
        self.api_request_response::<AdminUpdateUserRoute>(req).await
    }

    pub async fn admin_delete_user(&self, req: &AdminDeleteUserRequest) -> FrontendResult<()> {
        self.api_request::<AdminDeleteUserRoute>(req).await
    }

    // --- Public product browsing ---

    pub async fn product_list(
        &self,
        req: &ProductListRequest,
    ) -> FrontendResult<ProductListResponse> {
        self.api_request_response::<ProductListRoute>(req).await
    }

    pub async fn product_detail(
        &self,
        req: &ProductDetailRequest,
    ) -> FrontendResult<ProductDetailResponse> {
        self.api_request_response::<ProductDetailRoute>(req).await
    }

    pub async fn product_categories(&self) -> FrontendResult<ProductCategoriesResponse> {
        self.api_response::<ProductCategoriesRoute>().await
    }

    pub async fn product_brands(&self) -> FrontendResult<ProductBrandsResponse> {
        self.api_response::<ProductBrandsRoute>().await
    }

    // --- Admin product CRUD ---

    pub async fn admin_list_products(
        &self,
        req: &AdminListProductsRequest,
    ) -> FrontendResult<AdminListProductsResponse> {
        self.api_request_response::<AdminListProductsRoute>(req)
            .await
    }

    pub async fn admin_create_product(
        &self,
        req: &AdminCreateProductRequest,
    ) -> FrontendResult<AdminCreateProductResponse> {
        self.api_request_response::<AdminCreateProductRoute>(req)
            .await
    }

    pub async fn admin_update_product(
        &self,
        req: &AdminUpdateProductRequest,
    ) -> FrontendResult<AdminUpdateProductResponse> {
        self.api_request_response::<AdminUpdateProductRoute>(req)
            .await
    }

    pub async fn admin_delete_product(
        &self,
        req: &AdminDeleteProductRequest,
    ) -> FrontendResult<()> {
        self.api_request::<AdminDeleteProductRoute>(req).await
    }

    // --- Admin category CRUD ---

    pub async fn admin_list_categories(&self) -> FrontendResult<AdminListCategoriesResponse> {
        self.api_response::<AdminListCategoriesRoute>().await
    }

    pub async fn admin_create_category(
        &self,
        req: &AdminCreateCategoryRequest,
    ) -> FrontendResult<AdminCreateCategoryResponse> {
        self.api_request_response::<AdminCreateCategoryRoute>(req)
            .await
    }

    pub async fn admin_update_category(
        &self,
        req: &AdminUpdateCategoryRequest,
    ) -> FrontendResult<AdminUpdateCategoryResponse> {
        self.api_request_response::<AdminUpdateCategoryRoute>(req)
            .await
    }

    pub async fn admin_delete_category(
        &self,
        req: &AdminDeleteCategoryRequest,
    ) -> FrontendResult<()> {
        self.api_request::<AdminDeleteCategoryRoute>(req).await
    }

    // --- Admin brand CRUD ---

    pub async fn admin_list_brands(
        &self,
        req: &AdminListBrandsRequest,
    ) -> FrontendResult<AdminListBrandsResponse> {
        self.api_request_response::<AdminListBrandsRoute>(req).await
    }

    pub async fn admin_create_brand(
        &self,
        req: &AdminCreateBrandRequest,
    ) -> FrontendResult<AdminCreateBrandResponse> {
        self.api_request_response::<AdminCreateBrandRoute>(req)
            .await
    }

    pub async fn admin_update_brand(
        &self,
        req: &AdminUpdateBrandRequest,
    ) -> FrontendResult<AdminUpdateBrandResponse> {
        self.api_request_response::<AdminUpdateBrandRoute>(req)
            .await
    }

    pub async fn admin_delete_brand(&self, req: &AdminDeleteBrandRequest) -> FrontendResult<()> {
        self.api_request::<AdminDeleteBrandRoute>(req).await
    }
}
