# API Route System

This document describes the methodology for adding new API routes. The system should guarantee compile-time synchronization between route definitions, request/response types, handler implementations, and frontend callers through Rust's trait system.

## Architecture Overview

The API is defined across four packages:

| Package | Role |
|---|---|
| `groupshop-backend-shared` | Route enums, trait impls, request/response types, and shared API contracts |
| `api` (backend) | Handlers, DB access, durable object state |
| `frontend-shared` | Client caller methods that invoke the routes |
| `cloudflare/api/db/migrations/` | SQL table definitions (D1/SQLite) |

A route is fully defined when it has:
1. An enum variant in `groupshop-backend-shared` with `auth_requirement()` and `role_requirement()`
2. A route struct implementing one of the `ApiRoute*` traits
3. A handler function in the backend
4. A convenience method on `ApiClient` in the frontend
5. Any necessary DB structs and SQL migrations

## Route Trait Selection

Every route must implement exactly ONE of these traits (defined in `groupshop-backend-shared/src/route.rs`):

| Trait | Has Request Body | Has Response Body | Route | When to Use |
|---|---|---|---|---|
| `ApiRouteRequestResponse` | Yes (`Req`) | Yes (`Res`) | `const ROUTE` | Most CRUD operations — client sends data, server responds with data |
| `ApiRouteRequest` | Yes (`Req`) | No | `const ROUTE` | Fire-and-forget with payload |
| `ApiRouteResponse` | No | Yes (`Res`) | `const ROUTE` | Data fetches |
| `ApiRouteEmpty` | No | No | `const ROUTE` | Side-effect-only |
| `ApiRouteDynRequest` | No | Yes (`Res`) | `fn route()` | Dynamic URL with response |
| `ApiRouteDynEmpty` | No | No | `fn route()` | Dynamic URL, side-effect-only |

**Static vs. Dynamic:** Most routes use `const ROUTE` (static). Use the `Dyn` variants when the route URL itself depends on runtime data.

## Naming Conventions

### Route Structs

Format: `{Domain}{Feature}{Action}Route`

```text
DealJoinRoute
DealRefundRoute
AuthEmailPasswordRegisterRoute
ProductListRoute
```

### Request / Response Structs

Format: `{Domain}{Feature}{Action}Request` / `{Domain}{Feature}{Action}Response`

Drop the "Route" suffix — the `Request`/`Response` suffix replaces it:

```text
DealJoinRequest        / DealJoinResponse
AuthEmailPasswordRegisterRequest
ProductListResponse
```

### Route Enum Variants

Use PascalCase action names without the domain prefix:

```rust
pub enum ApiDealRoute {
    Detail,
    Join,
    Refund,
}
```

### URL Paths

Kebab-case, nested by domain:

```text
/deal/detail
/deal/join
/product/list
/auth/email-password/register
```

### All types use `#[groupshop]`

The `#[groupshop]` derive macro adds `Serialize`, `Deserialize`, `Debug`, `Clone` and applies `#[serde(rename_all = "snake_case")]` (or `#[serde(transparent)]` for newtype wrappers). Use it on all route structs, request/response types, and shared data types.

### ID fields use real types

All request/response structs must use typed IDs (`ProductId`, `DealId`, `UserId`, etc.) — never raw `String`.

## Auth and Role Requirements

Every route domain enum must implement two methods:

```rust
impl ApiDealRoute {
    pub fn auth_requirement(&self) -> Option<AuthRequirement> {
        // None           → public endpoint, no auth needed
        // Some(Session)  → requires valid session token
        // Some(Refresh)  → requires refresh-cookie flow
    }

    pub fn role_requirement(&self) -> Option<Vec<UserRole>> {
        // None              → no role check
        // Some(vec![...])   → user must hold ALL listed roles
    }
}
```

The top-level `ApiRoute` delegates to sub-route enums. For entire domains that share auth/role requirements, the delegation is handled at the `ApiRoute` level.

## File Locations

### 1. Route Definition (`groupshop-backend-shared`)

```text
packages/backend/backend-shared/src/
├── route.rs
└── route/
    ├── auth.rs
    ├── deal.rs
    ├── product.rs
    └── user.rs
```

Each domain has its own sub-module. Route structs, request types, and response types live in the same file when they belong to the same feature.

### 2. Backend Handlers

```text
packages/backend/api/src/
├── handler.rs
└── handler/
    ├── auth/
    ├── deal/
    ├── product/
    └── user/
```

Handler functions follow the signature:

```rust
pub async fn handle_feature_action(
    ctx: &mut ApiContext,
    req: HttpRequest,
) -> ApiResult<ResponseType>
```

The first line of every handler that accepts a JSON body should shadow `req` with the deserialized type:

```rust
let req: MyRequestType = req_to_json(req).await?;
```

Do not use a different name like `body`.

### 3. Backend Database Access

```text
packages/backend/api/src/
├── db/
│   ├── mod.rs
│   ├── tables.rs
│   ├── auth/
│   ├── deal/
│   ├── product/
│   └── user/
```

**DB struct naming:** `{Table}Db`

**Table name constants** live in `db/tables.rs` and must stay in sync with the SQL migrations.

### 4. Backend Durable Object State

```text
packages/backend/api/src/
├── durable/
│   ├── auth/
│   └── deal/
```

Use durable objects for state that must be managed atomically and cannot rely on D1's eventual consistency.

### 5. SQL Migrations

```text
cloudflare/api/db/migrations/
├── 0001_initial.sql
├── 0002_*.sql
...
```

During development, edit migration files **in place** and run `task db:recreate-dev`.

### 6. Frontend Client Callers

```text
packages/frontend/frontend-shared/src/api/
├── client.rs
└── client/
    ├── auth.rs
    ├── deal.rs
    ├── product.rs
    └── user.rs
```

Each domain gets its own file under `client/` with an `impl ApiClient` block containing convenience methods.

The generic callers should look like:

```rust
api_response::<T: ApiRouteResponse>()                     -> FrontendResult<T::Res>
api_request_response::<T: ApiRouteRequestResponse>(req)   -> FrontendResult<T::Res>
api_request::<T: ApiRouteRequest>(req)                    -> FrontendResult<()>
api_empty::<T: ApiRouteEmpty>()                           -> FrontendResult<()>
```

## Checklist: Adding a New Route

### 1. Define the SQL schema (if needed)

- [ ] Edit the appropriate migration file in `cloudflare/api/db/migrations/`
- [ ] Run `task db:recreate-dev` to apply changes
- [ ] If adding seed data, ensure values match any corresponding Rust enums

### 2. Define the route in `groupshop-backend-shared`

- [ ] Add an enum variant to the appropriate `Api{Domain}Route` enum
- [ ] Add the variant to the `Display` impl
- [ ] Add the variant to the URL parsing impl
- [ ] Add the variant to `auth_requirement()` and `role_requirement()`
- [ ] Create a route struct: `{Domain}{Feature}{Action}Route`
- [ ] Implement the appropriate `ApiRoute*` trait on the struct
- [ ] Create request/response structs as needed, all with `#[groupshop]`

### 3. Add the DB access layer (if needed)

- [ ] Add a table name constant in `api/src/db/tables.rs`
- [ ] Create a `{Table}Db` struct in the appropriate `api/src/db/{domain}/` file
- [ ] Implement query methods (`load_by_*`, `insert`, `update_*`, `exists`, etc.)
- [ ] Place helper functions in the same file

### 4. Implement the backend handler

- [ ] Create a handler function in `api/src/handler/{domain}/{feature}.rs`
- [ ] Wire it into the `handle_route()` match in `api/src/handler.rs`
- [ ] Shadow `req` with the deserialized body: `let req: MyRequestType = req_to_json(req).await?;`
- [ ] Return the response type directly

### 5. Add the frontend caller

- [ ] Add a convenience method to `impl ApiClient` in `frontend-shared/src/api/client/{domain}.rs`
- [ ] Call the appropriate generic method (`api_request_response`, `api_response`, etc.)

### 6. Verify

- [ ] Run `task lint`
- [ ] If route types changed, both frontend and backend must compile together
