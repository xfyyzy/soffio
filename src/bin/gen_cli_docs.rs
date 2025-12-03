//! Generator for CLI documentation (English + Chinese) with an auto-built command matrix.
//! Run with `cargo run --bin gen-cli-docs`.

#![deny(clippy::all, clippy::pedantic)]

use std::fs;

use clap::CommandFactory;

#[path = "soffio_cli/args.rs"]
mod cli;
use cli::Cli;

struct Row {
    path: String,
    about: String,
}

fn collect_rows(cmd: &clap::Command, prefix: &str, rows: &mut Vec<Row>) {
    let name = cmd.get_name();
    let path = if prefix.is_empty() {
        name.to_string()
    } else {
        format!("{prefix} {name}")
    };
    let about = cmd
        .get_about()
        .map_or_else(String::new, ToString::to_string);
    rows.push(Row {
        path: path.clone(),
        about,
    });
    for sub in cmd.get_subcommands() {
        collect_rows(sub, &path, rows);
    }
}

fn table(rows: &[Row]) -> String {
    use std::fmt::Write;

    let mut out = String::from("| Command | Description |\n|---|---|\n");
    for row in rows {
        let about = if row.about.is_empty() {
            String::new()
        } else {
            row.about.replace('|', "\\|")
        };
        writeln!(&mut out, "| `{}` | {} |", row.path, about).expect("write string");
    }
    out
}

fn render_en(matrix: &str) -> String {
    format!(
        "# soffio-cli\n\nEnglish | [中文](cli.zh.md)\n\nA headless admin CLI for Soffio. Built to cover every admin scenario (posts, pages, tags, navigation, uploads, settings, jobs, audit, API key introspection) via the headless HTTP API.\n\n## Quick start\n```
cargo build --release --bin soffio-cli
SOFFIO_SITE_URL=https://your.site \\
SOFFIO_API_KEY_FILE=~/.config/soffio/key \\
./target/release/soffio-cli api-keys me
```\n\n## Global options\n- `--site` (env `SOFFIO_SITE_URL`), required.\n- `--key-file` (env `SOFFIO_API_KEY_FILE`), file wins over env.\n- API key is **env only** (`SOFFIO_API_KEY`); no CLI flag to avoid shell history leaks.\n- `--help` / `--version` available everywhere.\n\n## Long text input\nUse file flags to avoid shell quoting limits: `--body-file`, `--summary-file`, `--description-file`, `--favicon-svg-file`. The file content is read verbatim.\n\n## Command matrix (generated)\n{matrix}\n\n## Typical admin scenarios\n- Inspect current API key: `soffio-cli api-keys me`\n- Create a post from files: `soffio-cli posts create --title \"Title\" --excerpt \"Short\" --body-file post.md --summary-file summary.md --status published`\n- Patch a page body: `soffio-cli pages patch-body --id <UUID> --body-file page.md`\n- Upload an asset: `soffio-cli uploads upload ./image.png`\n- List jobs with filter: `soffio-cli jobs list --state queued --job-type send_email`\n- Audit search: `soffio-cli audit list --actor alice --action update_post`\n\n## Safety notes\n- Prefer key files and env vars; never paste keys on the command line.\n- CLI is stateless; retry on transport errors. Outputs are JSON pretty-printed for piping to `jq`.\n",
    )
}

fn render_zh(matrix: &str) -> String {
    format!(
        "# soffio-cli\n\n[English](cli.md) | 中文\n\nSoffio 的 headless 管理 CLI，覆盖帖子、页面、标签、导航、上传、站点设置、后台任务、审计等管理场景。\n\n## 快速开始\n```
cargo build --release --bin soffio-cli
SOFFIO_SITE_URL=https://your.site \\
SOFFIO_API_KEY_FILE=~/.config/soffio/key \\
./target/release/soffio-cli api-keys me
```\n\n## 全局选项\n- `--site`（或环境变量 `SOFFIO_SITE_URL`），必填。\n- `--key-file`（或 `SOFFIO_API_KEY_FILE`），文件优先。\n- API 密钥只能通过环境变量 `SOFFIO_API_KEY` 提供，出于安全考虑不提供 CLI 旗标。\n- 所有命令都支持 `--help` / `--version`。\n\n## 长文本输入\n通过文件参数避免转义问题：`--body-file`、`--summary-file`、`--description-file`、`--favicon-svg-file`，文件内容将被原样读取。\n\n## 命令矩阵（自动生成）\n{matrix}\n\n## 常用场景示例\n- 查看当前密钥信息：`soffio-cli api-keys me`\n- 从文件创建文章：`soffio-cli posts create --title \"标题\" --excerpt \"摘要\" --body-file post.md --summary-file summary.md --status published`\n- 更新页面正文：`soffio-cli pages patch-body --id <UUID> --body-file page.md`\n- 上传资源：`soffio-cli uploads upload ./image.png`\n- 按状态查看任务：`soffio-cli jobs list --state running`\n- 检索审计日志：`soffio-cli audit list --actor admin --action delete_post`\n\n## 安全提示\n- 使用密钥文件 / 环境变量，避免在命令行暴露密钥。\n- CLI 无状态，如遇网络错误可重试；输出为 JSON，便于 `jq` 等工具处理。\n",
    )
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut rows = Vec::new();
    let cmd = Cli::command();
    collect_rows(&cmd, "", &mut rows);
    let matrix = table(&rows);

    let en = render_en(&matrix);
    let zh = render_zh(&matrix);

    fs::write("docs/cli.md", en)?;
    fs::write("docs/cli.zh.md", zh)?;

    println!("Generated docs/cli.md and docs/cli.zh.md");
    Ok(())
}
