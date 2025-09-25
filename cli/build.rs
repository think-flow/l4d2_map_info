use std::env;
use std::fs;
use std::path::Path;

fn main() {
    // 获取项目根目录路径
    let project_root = env::var("CARGO_MANIFEST_DIR").expect("Failed to get manifest dir");
    
    // 定义源文件路径（项目根目录下的 vpkinfo.dll）
    let source_dll = Path::new(&project_root).join("vpkinfo.dll");
    
    // 获取 Cargo 设置的输出目录路径（如 target/debug）
    let out_dir = env::var("OUT_DIR").expect("Failed to get OUT_DIR");
    // 注意：OUT_DIR 通常指向更深层的目录，我们需要向上一级到二进制文件所在目录
    let target_dir = Path::new(&out_dir).parent().unwrap().parent().unwrap().parent().unwrap();
    
    // 定义目标文件路径
    let target_dll = target_dir.join("vpkinfo.dll");
    
    // 检查源文件是否存在
    if !source_dll.exists() {
        // 如果不存在，可以打印一个警告信息（这不会导致编译失败）
        println!("cargo:warning=vpkinfo.dll not found at {:?}, skipping copy.", source_dll);
        return;
    }
    
    // 执行复制操作
    match fs::copy(&source_dll, &target_dll) {
        Ok(_) => {
            // 通知 Cargo 如果源 DLL 文件发生变化，需要重新运行此构建脚本
            println!("cargo:rerun-if-changed={}", source_dll.to_str().unwrap());
            println!("cargo:warning=Successfully copied vpkinfo.dll to output directory.");
        }
        Err(e) => {
            // 如果复制失败，打印错误信息（但不会中止整个编译过程）
            println!("cargo:warning=Failed to copy vpkinfo.dll: {}", e);
        }
    }
}