use crate::app::builder::run_tauri_app;
use anyhow::Result;

/// 处理命令行参数
pub fn handle_cli_args() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();

    match args.len() {
        // 无参数：正常启动GUI
        1 => {
            run_tauri_app();
        }
        // 单参数：帮助或版本
        2 => {
            match args[1].as_str() {
                "--help" | "-h" => print_help(),
                "--version" | "-v" => print_version(),
                _ => {
                    eprintln!("未知参数: {}", args[1]);
                    print_help();
                    std::process::exit(1);
                }
            }
        }
        // 多参数：MCP请求模式
        _ => {
            if args[1] == "--mcp-request" && args.len() >= 3 {
                handle_mcp_request(&args[2])?;
            } else {
                eprintln!("无效的命令行参数");
                print_help();
                std::process::exit(1);
            }
        }
    }

    Ok(())
}

/// 处理MCP请求
fn handle_mcp_request(_request_file: &str) -> Result<()> {
    // 启动GUI处理弹窗
    run_tauri_app();
    Ok(())
}

/// 显示帮助信息
fn print_help() {
    println!("寸止 - 智能代码审查工具");
    println!();
    println!("用法:");
    println!("  等一下                    启动设置界面");
    println!("  等一下 --mcp-request <文件>  处理 MCP 请求");
    println!("  等一下 --help             显示此帮助信息");
    println!("  等一下 --version          显示版本信息");
}

/// 显示版本信息
fn print_version() {
    println!("寸止 v{}", env!("CARGO_PKG_VERSION"));
}
