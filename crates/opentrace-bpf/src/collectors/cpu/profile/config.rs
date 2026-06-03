// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.
// 面向用户配置文件
#[derive(Default)]
pub struct Config {
    // pid 和 tid 只能二选一，设置了pid 就不能设置tid, 设置了tid就不能设置pid
    // 如果两个都不设置，则默认为0
    // 基于pid 过滤, 会捕获进程号是pid下面所有线程的event事件
    // 不填/none
    pub pid: i32,
    // 基于tid 过滤，可以指定一个或者多个
    // tid必须 > 0
    pub tids: Option<Vec<u32>>,
    // cpu -1 代表所有的cpu, >=0 代表指定的cpu
    pub cpu: i32,
    pub group_id: i32,

    pub custom_btf_path: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_has_zero_pid() {
        let config = Config::default();
        assert_eq!(config.pid, 0);
    }

    #[test]
    fn default_config_has_no_tids() {
        let config = Config::default();
        assert!(config.tids.is_none());
    }

    #[test]
    fn default_config_has_zero_cpu() {
        let config = Config::default();
        assert_eq!(config.cpu, 0);
    }

    #[test]
    fn default_config_has_zero_group_id() {
        let config = Config::default();
        assert_eq!(config.group_id, 0);
    }

    #[test]
    fn default_config_has_no_custom_btf_path() {
        let config = Config::default();
        assert!(config.custom_btf_path.is_none());
    }
}
