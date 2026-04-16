use groupshop_frontend_shared::window;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Route {
    Index,
    Users,
    Products,
    Categories,
    Brands,
}

impl Route {
    pub fn current() -> Self {
        let path = window()
            .location()
            .pathname()
            .unwrap_or_else(|_| "/".to_string());

        let parts = path
            .trim_matches('/')
            .split('/')
            .filter(|segment| !segment.is_empty())
            .collect::<Vec<_>>();

        match parts.as_slice() {
            [] => Self::Index,
            ["users"] => Self::Users,
            ["products"] => Self::Products,
            ["categories"] => Self::Categories,
            ["brands"] => Self::Brands,
            _ => Self::Index,
        }
    }

    pub fn link(&self) -> String {
        match self {
            Self::Index => "/".to_string(),
            Self::Users => "/users".to_string(),
            Self::Products => "/products".to_string(),
            Self::Categories => "/categories".to_string(),
            Self::Brands => "/brands".to_string(),
        }
    }

    pub fn go_to_url(&self) {
        let _ = window().location().set_href(&self.link());
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::Index => "Dashboard",
            Self::Users => "Users",
            Self::Products => "Products",
            Self::Categories => "Categories",
            Self::Brands => "Brands",
        }
    }
}
