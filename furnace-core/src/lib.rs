pub fn add(left: u64, right: u64) -> u64 {
    left + right
}

pub mod recipe;
pub mod services;
pub mod php;
pub mod nginx_util;
pub mod web_service;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}
