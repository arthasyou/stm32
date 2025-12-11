pub fn main() {
    // defmt 配置
    println!("cargo:rerun-if-env-changed=DEFMT_LOG");
    println!("cargo:rustc-link-arg-bins=-Tdefmt.x");
    println!("cargo:rustc-link-arg-bins=--nmagic");
    println!("cargo:rustc-link-arg-bins=-Tlink.x");

    // 从 .proto 文件生成 Rust 代码
    let mut config = prost_build::Config::new();

    // 为 no_std 环境配置
    config.btree_map(&["."]);  // 使用 BTreeMap 而不是 HashMap

    // 生成代码
    config
        .compile_protos(&["proto/coin_pusher.proto"], &["proto/"])
        .unwrap();

    println!("cargo:rerun-if-changed=proto/coin_pusher.proto");
}