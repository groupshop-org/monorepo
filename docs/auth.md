# High-level

The API is secured by requiring 2 pieces of information on each request:

1. A valid session header token
2. A valid session cookie token

Anti-CSRF: Since the header token is not stored in a cookie, it cannot be forged by tricking the user into visiting a malicious website which reads cookies.

Anti-Exfiltration (e.g. via XSS): Since the session cookie is inaccessible to javascript (Http-Only, Secure, Domain-restricted, SameSite=Strict), it cannot be exfiltrated from JS. However, XSS can still be used to make requests with the victim's session cookie, so additional defenses such as Content Security Policy (CSP) and Subresource Integrity (SRI) should be used to mitigate this risk.

Since this is the typical "hot path", these tokens are verified on the backend simply by cryptographically verifying the signature (HMAC) of the token, without any database lookups. This allows for very fast verification on each request (using WebCrypto in the worker and key from Secrets).

# User Flow - Login

Upon initial login:

1. A new, random "refresh id" is created on the backend
2. A new, random "refresh token" is created on the backend and stored in a Durable Object keyed by the refresh id
3. The refresh token is set in the user's cookie's (Http-Only, Secure, Domain-restricted, SameSite=Strict)

This *refresh* cookie token is different than the session tokens, and remains in the browser across multiple tabs and browser restarts. Since each login issues a distinct token, multiple devices can be active simultaneously without affecting eachother.

# User Flow - Token Session

The client requests session tokens like this:

1. The client makes a POST request to the token session endpoint, including the refresh token in the request (since it's a cookie, it will be included automatically by the browser).
2. The backend verifies the refresh token, and generates new session tokens for cookie and header. It signs these session tokens with a key (stored in Worker Secrets) then returns the header token in the response body and sets the cookie token in the response.
3. The client stores the new header token in memory (not storage).
4. Subsequent requests must include the header token; the new session cookie token will be included automatically by the browser.

Security note: the session endpoint *can* be executed by a CSRF attack, but it doesn't really do much since the attacker won't see the returned header token, and the side-effects are just signing over random bytes.

Expiration: Session tokens are short-lived (e.g. 15 minutes) and are not stored in the DO, so they cannot be revoked. Due to the short-lived expiration, the API must be able to tolerate expired tokens and automatically refresh them when necessary (e.g. client code will need "middleware" to retry a request after refreshing).

Refresh: when the session endpoint is hit, it also updates the refresh token. Specifically, the backend changes the refresh token (generating+storing a new random token and marking the old one as "expiring"), and sets the new token in the response cookie. This ensures that refresh tokens are rotated often, limiting the window of opportunity for an attacker to use a stolen refresh token. The old refresh token may still be used up to the expiration time, so we can support the use-case of multiple tabs being opened at once. If the old refresh token is used within its expiration window, then a new refresh token is NOT generated, rather the existing refresh token is reused. This allows for multiple tabs to be opened at once without causing token churn, while still ensuring that refresh tokens are rotated regularly - and therefore this expiration time is more like a grace window, much shorter than the session token (e.g. 10 seconds)

# One-time-use tokens

These are used for things like "forgot password", "verify email", and similar out-of-band account actions. They are generated as random strings, stored in the DO, and sent to the user outside the normal authenticated session flow (email, notifications, etc). These tokens are automatically cleaned up via DO alarms.

Each one-time-use token gets its own Durable Object keyed by the token itself. When the user presents the token, the backend looks it up in the DO, performs the necessary action (e.g. reset password), then deletes the token from the DO to prevent reuse.

**Important**: Never call `auth_produce_once_token()` or `auth_consume_once_token()` directly in handlers. Always use:
- `sign_once_token(ctx, kind)` to produce — this wraps the token in a signed `AuthToken` envelope.
- `Validation::once_token_consume(ctx, &token)` to consume — this validates the signature before consuming.

This ensures every once token is cryptographically signed and verified, preventing forgery.

# Where it is set

1. On any request with "session" requirements, if the session is expired, all tokens are potentially updated
2. On any request with "refresh" requirements, the refresh token (but not session) will be refreshed
3. On explicit routes such as login or refresh

Since the session header may be regenerated at any time, clients must look for it in the appropriate header of the *response*

# Auth Routes

All auth routes are `POST` requests under `/auth/`.

## Email/Password

| Route | Auth | Description |
|---|---|---|
| `email-password/register` | None | Create account with email + password. Validates email format and password length (min 8 chars). Hashes password with Argon2. Creates user with `PartialRegistration` role and sends verification email. Signs user in on success. |
| `email-password/signin` | None | Sign in with email + password. Looks up email, verifies password hash. Returns `AuthError::InvalidCredentials` for both unknown email and wrong password (no user enumeration). Signs user in on success. |
| `email-password/send-reset-any` | None | Send password reset email for any email address. Silently succeeds if email is not registered (no user enumeration). Creates a one-time-use `ResetPassword` token. |
| `email-password/send-reset-me` | Session | Send password reset email for the currently signed-in user. **Not yet implemented.** |
| `email-password/confirm-reset` | None | Confirm password reset with one-time token and new password. Validates password length, consumes token, updates password hash, and signs user in. |

## Email Verification

| Route | Auth | Description |
|---|---|---|
| `email-address/send-verification` | Session | Send a verification email to the current user. Creates a one-time-use `VerifyEmail` token. |
| `email-address/confirm-verification` | None | Confirm email verification with one-time token. Adds `EmailVerified` role. |

## OpenID (Google)

| Route | Auth | Description |
|---|---|---|
| `openid-connect` | None | Start OAuth flow. Returns Google auth URL with PKCE challenge and a one-time state token. |
| `openid-access-token-hook/google` | None | Google redirect callback. Exchanges auth code for ID token, validates signature/claims, resolves to existing or new user, returns a finalize token. |
| `openid-finalize-query` | None | Peek at finalize token to check if the user already exists and their roles. |
| `openid-finalize-exec` | None | Consume finalize token. For existing users: links OpenID. For new users: registers with consent, links OpenID. Signs user in. |

## Session Management

| Route | Auth | Description |
|---|---|---|
| `refresh` | Refresh | Refresh session tokens using the refresh token cookie. |
| `signout` | Session | Clears refresh and session cookies (Max-Age=0). |

# Error Handling

Auth errors are returned as typed `AuthError` variants serialized to JSON. The frontend deserializes these back into typed errors so user-facing messages are clean (e.g. "invalid email or password" instead of raw HTTP/DB errors).

Key error variants:
- `InvalidCredentials` — wrong email or password (deliberately vague)
- `EmailAlreadyRegistered` — duplicate registration attempt
- `PasswordTooShort` — password under 8 characters
- `InvalidEmail` — malformed email address
- `OnceTokenMismatch` — wrong token type presented (e.g. verify token used for reset)
- `Expired` — token has expired

# Password Policy

- Minimum 8 characters
- Hashed with Argon2 using a random 20-byte salt
- Validation is centralized in `validate_password()` used by both registration and password reset

# Sign-out

Sign-out is handled by `ApiCtx::sign_out()` which all sign-out buttons call. The flow:

1. POST to `/auth/signout` — the server responds with `Set-Cookie` headers that clear both the refresh token and session cookie (Max-Age=0).
2. Clear the session header token from local storage.
3. Clear the in-memory profile so routing treats the user as logged out.
4. Navigate to the landing page.

The server call is fire-and-forget: if it fails (e.g. network offline), local cleanup still runs so the user is effectively signed out from the browser's perspective.
