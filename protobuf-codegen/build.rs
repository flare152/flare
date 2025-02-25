use std::path::PathBuf;

fn main() {
    // 定义.proto文件及其对应的输出目录
    let proto_files = vec![
        ("proto/net.proto", "src/flare_gen"),
       // ("proto/user.proto", "src/flare_gen/user"),
    ];

    // 配置 prost-build
    let mut config = prost_build::Config::new();

    for (proto, out_dir) in proto_files {
        let proto_dir = PathBuf::from("proto");
        let out_path = PathBuf::from(out_dir);

        // 创建输出目录
        std::fs::create_dir_all(&out_path).unwrap();

        // 设置输出目录
        config.out_dir(&out_path);

        // 编译 proto 文件
        config.compile_protos(&[proto], &[proto_dir]).unwrap();

        // 让 cargo 在 proto 文件改变时重新运行
        println!("cargo:rerun-if-changed={}", proto);
    }
}