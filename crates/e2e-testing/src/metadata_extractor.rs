use anyhow::Result;
use regex::Regex;
use url::Url;

pub struct AppRoute {
    pub name: String,
    pub route_url: String,
    pub wildcard: bool,
}

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

        return Err("").map_err(anyhow::Error::msg);
    }
}

impl Drop for AppMetadata {
    fn drop(&mut self) {
        print!("dropping app for {}", self.name)
    }
}

/// Extracts version of app being deployed by parsing logs
pub fn extract_version_from_logs(appname: &str, logs: &str) -> String {
    let re: Regex = Regex::new(format!("Uploading {} version (.*)...", appname).as_str()).unwrap();
    let v = match re.find(logs) {
        None => "",
        Some(v) => v.as_str(),
    };

    return v.to_string();
}

/// Extracts routes of app being deployed by parsing logs
pub fn extract_routes_from_logs(logs: &str) -> Vec<AppRoute> {
    let re: Regex = Regex::new(r##"^\s*(.*): (https?://[^\s^\\(]+)(.*)$"##).unwrap();
    let mut route_start = false;
    let lines = logs.split("\n");
    println!("{:?}", lines);
    let mut routes: Vec<AppRoute> = vec![];
    for line in lines {
        println!("line is {:?}", line);

        if line.trim() == "" {
            continue;
        }

        if !route_start && line.trim() != "Available Routes:" {
            continue;
        }

        if !route_start {
            route_start = true;
            continue;
        }

        println!("line is before capturing {:?}", line);
        let captures = re.captures(line).unwrap();

        let route = AppRoute {
            name: captures.get(1).unwrap().as_str().to_string(),
            route_url: captures.get(2).unwrap().as_str().to_string(),
            wildcard: captures.get(3).unwrap().as_str().to_string() == "(wildcard)",
        };

        routes.push(route)
    }

    return routes;
}

/// Extract metadata of app being deployed
///
/// name, base url, version and available app routes are extracted from logs
pub fn extract_app_metadata_from_logs(appname: &str, logs: &str) -> AppMetadata {
    let version = extract_version_from_logs(appname, logs);
    let app_routes = extract_routes_from_logs(logs);
    let mut base = "".to_string();
    if app_routes.len() > 0 {
        base = match Url::parse(&app_routes.first().unwrap().route_url) {
            Err(err) => panic!("{}", err),
            Ok(url) => format!("{}://{}", url.scheme(), url.host().unwrap().to_string()),
        }
    }
    return AppMetadata {
        name: appname.to_string(),
        base,
        version,
        app_routes,
    };
}
