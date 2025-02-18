use std::path::PathBuf;

fn main() {
    let proto_files = vec!["proto/net.proto"];
    let proto_dir = PathBuf::from("proto");
    let out_dir = PathBuf::from("./src/flare_gen");

    // 创建输出目录
    std::fs::create_dir_all(&out_dir).unwrap();

    // 配置 prost-build
    let mut config = prost_build::Config::new();
    config.out_dir(&out_dir);
    
    // 编译 proto 文件
    config.compile_protos(&proto_files, &[proto_dir]).unwrap();

    // 让 cargo 在 proto 文件改变时重新运行
    for proto in proto_files {
        println!("cargo:rerun-if-changed={}", proto);
    }
} 