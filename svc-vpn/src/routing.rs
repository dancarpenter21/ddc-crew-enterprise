use std::net::IpAddr;

use ipnet::IpNet;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Route {
    pub peer_index: usize,
    pub cidr: IpNet,
}

#[derive(Debug, Clone, Default)]
pub struct AllowedIps {
    routes: Vec<Route>,
}

impl AllowedIps {
    pub fn new(routes: Vec<Route>) -> Self {
        let mut routes = routes;
        routes.sort_by_key(|route| std::cmp::Reverse(route.cidr.prefix_len()));
        Self { routes }
    }

    pub fn lookup(&self, addr: IpAddr) -> Option<usize> {
        self.routes
            .iter()
            .find(|route| route.cidr.contains(&addr))
            .map(|route| route.peer_index)
    }
}

#[cfg(test)]
mod tests {
    use std::net::IpAddr;

    use super::*;

    #[test]
    fn uses_longest_prefix_match() {
        let table = AllowedIps::new(vec![
            Route {
                peer_index: 1,
                cidr: "10.0.0.0/8".parse().unwrap(),
            },
            Route {
                peer_index: 2,
                cidr: "10.1.2.3/32".parse().unwrap(),
            },
        ]);

        assert_eq!(table.lookup("10.1.2.3".parse::<IpAddr>().unwrap()), Some(2));
        assert_eq!(table.lookup("10.2.0.1".parse::<IpAddr>().unwrap()), Some(1));
    }
}
