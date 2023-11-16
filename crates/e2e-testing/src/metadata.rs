use anyhow::Result;

#[derive(Clone)]
pub struct AppRoute {
    pub name: String,
    pub route_url: String,
    pub wildcard: bool,
}

#[derive(Clone)]
pub struct AppMetadata {
    pub name: String,
    pub base: String,
    pub app_routes: Vec<AppRoute>,
    pub version: String,
}

impl AppMetadata {
    pub fn get_route_with_name(&self, name: String) -> Result<&AppRoute> {
        for route in &self.app_routes {
            if route.name == name {
                return Ok(route);
            }
        }

        Err("requested route not found").map_err(anyhow::Error::msg)
    }
}
