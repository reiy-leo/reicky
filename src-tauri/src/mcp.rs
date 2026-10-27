//! MCP 服务（stdio）：`reicky --mcp` 启动，让 AI 客户端读写便签。
//!
//! 不开窗口，直接读写和应用同一个数据目录。应用开着时会在 1~2 秒内发现文件变化并刷新窗口
//! （见 lib.rs 的 `watch_notes`）。和应用一样只归档、不删除；已归档的便签只读，要先恢复。
//!
//! 协议：每行一条 JSON-RPC 2.0 消息。只实现 initialize / ping / tools/list / tools/call。

use crate::store::{Note, Store};
use crate::COLORS;
use serde_json::{json, Value};
use std::io::{BufRead, Write};
use std::path::PathBuf;

const IDENTIFIER: &str = "dev.rei8t.reicky";
const DEFAULT_PROTOCOL: &str = "2025-06-18";

/// 和 Tauri 的 app_data_dir 一致：系统数据目录 + identifier
fn data_dir() -> Result<PathBuf, String> {
    dirs::data_dir()
        .map(|d| d.join(IDENTIFIER))
        .ok_or_else(|| "找不到系统数据目录".to_string())
}

pub fn serve() {
    let store = match data_dir().and_then(Store::open) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("reicky --mcp: {e}");
            std::process::exit(1);
        }
    };
    let stdin = std::io::stdin();
    let mut out = std::io::stdout();
    for line in stdin.lock().lines() {
        let Ok(line) = line else { break };
        if line.trim().is_empty() {
            continue;
        }
        let reply = match serde_json::from_str::<Value>(&line) {
            Ok(msg) => handle(&store, &msg),
            // -32700：JSON 解析失败
            Err(e) => Some(error(Value::Null, -32700, &format!("JSON 解析失败：{e}"))),
        };
        if let Some(r) = reply {
            let _ = writeln!(out, "{r}");
            let _ = out.flush();
        }
    }
}

fn error(id: Value, code: i64, msg: &str) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": msg } })
}

fn ok(id: Value, result: Value) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "result": result })
}

/// 通知（没有 id）不回复
fn handle(store: &Store, msg: &Value) -> Option<Value> {
    let id = msg.get("id").cloned()?;
    let method = msg.get("method").and_then(Value::as_str).unwrap_or("");
    let params = msg.get("params").cloned().unwrap_or(Value::Null);
    Some(match method {
        "initialize" => {
            let ver = params
                .get("protocolVersion")
                .and_then(Value::as_str)
                .unwrap_or(DEFAULT_PROTOCOL);
            ok(
                id,
                json!({
                    "protocolVersion": ver,
                    "capabilities": { "tools": {} },
                    "serverInfo": { "name": "reicky", "version": env!("CARGO_PKG_VERSION") },
                    "instructions": "reicky 是桌面便签。便签只能归档不能删除；已归档的便签只读，先 restore_note 才能改。",
                }),
            )
        }
        "ping" => ok(id, json!({})),
        "tools/list" => ok(id, json!({ "tools": tools() })),
        "tools/call" => {
            let name = params.get("name").and_then(Value::as_str).unwrap_or("");
            let args = params.get("arguments").cloned().unwrap_or(json!({}));
            match call(store, name, &args) {
                Ok(v) => ok(
                    id,
                    json!({ "content": [{ "type": "text", "text": v.to_string() }], "structuredContent": v }),
                ),
                // 工具执行出错按 MCP 约定放在 result 里，让模型能看到原因
                Err(e) => ok(
                    id,
                    json!({ "content": [{ "type": "text", "text": e }], "isError": true }),
                ),
            }
        }
        // -32601：方法不存在
        _ => error(id, -32601, &format!("不支持的方法：{method}")),
    })
}

fn tools() -> Value {
    let id = json!({ "type": "string", "description": "便签 id" });
    let color = json!({ "type": "string", "enum": COLORS });
    json!([
        {
            "name": "list_notes",
            "description": "按修改时间倒序列出便签，可按文字/路径搜索、按颜色筛选。返回每条的 id、颜色、首行摘要、修改时间。空白便签不列出。",
            "inputSchema": { "type": "object", "properties": {
                "query": { "type": "string", "description": "文字或路径包含（不分大小写）" },
                "color": color,
                "archived": { "type": "boolean", "description": "true 只看已归档，默认 false" },
                "limit": { "type": "integer", "minimum": 1, "maximum": 100, "description": "默认 20" },
                "offset": { "type": "integer", "minimum": 0 }
            } }
        },
        {
            "name": "get_note",
            "description": "读一张便签的全文、颜色、拖入的路径和状态。",
            "inputSchema": { "type": "object", "properties": { "id": id }, "required": ["id"] }
        },
        {
            "name": "create_note",
            "description": "新建一张便签。应用开着时会出现在贴纸列表最上面。",
            "inputSchema": { "type": "object", "properties": {
                "text": { "type": "string" },
                "color": color
            }, "required": ["text"] }
        },
        {
            "name": "update_note",
            "description": "用新文字整体替换一张便签的内容。",
            "inputSchema": { "type": "object", "properties": { "id": id, "text": { "type": "string" } }, "required": ["id", "text"] }
        },
        {
            "name": "append_note",
            "description": "在便签末尾追加文字（自动换行）。",
            "inputSchema": { "type": "object", "properties": { "id": id, "text": { "type": "string" } }, "required": ["id", "text"] }
        },
        {
            "name": "set_color",
            "description": "改便签颜色。",
            "inputSchema": { "type": "object", "properties": { "id": id, "color": color }, "required": ["id", "color"] }
        },
        {
            "name": "archive_note",
            "description": "归档便签（reicky 没有删除，归档后只读，可恢复）。",
            "inputSchema": { "type": "object", "properties": { "id": id }, "required": ["id"] }
        },
        {
            "name": "restore_note",
            "description": "把已归档的便签恢复成可编辑。",
            "inputSchema": { "type": "object", "properties": { "id": id }, "required": ["id"] }
        }
    ])
}

fn s<'a>(args: &'a Value, key: &str) -> Result<&'a str, String> {
    args.get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("缺少参数 {key}"))
}

fn check_color(c: &str) -> Result<(), String> {
    if COLORS.contains(&c) {
        Ok(())
    } else {
        Err(format!("不支持的颜色：{c}，可选 {}", COLORS.join(", ")))
    }
}

fn full(n: &Note) -> Value {
    json!({
        "id": n.id, "text": n.text, "color": n.color, "paths": n.paths,
        "archived": n.archived, "created": n.created, "modified": n.modified,
    })
}

/// 改之前确认没归档
fn editable(store: &Store, id: &str) -> Result<(), String> {
    if store.get(id)?.archived {
        return Err("这张便签已归档，只读。先用 restore_note 恢复。".into());
    }
    Ok(())
}

fn call(store: &Store, name: &str, args: &Value) -> Result<Value, String> {
    match name {
        "list_notes" => {
            let color = args.get("color").and_then(Value::as_str);
            if let Some(c) = color {
                check_color(c)?;
            }
            let limit = args
                .get("limit")
                .and_then(Value::as_u64)
                .unwrap_or(20)
                .clamp(1, 100) as usize;
            let offset = args.get("offset").and_then(Value::as_u64).unwrap_or(0) as usize;
            let archived = args
                .get("archived")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            let page = store.list(
                offset,
                limit,
                archived,
                args.get("query").and_then(Value::as_str),
                color,
            );
            Ok(json!({ "notes": page.items, "hasMore": page.has_more }))
        }
        "get_note" => Ok(full(&store.get(s(args, "id")?)?)),
        "create_note" => {
            let color = args.get("color").and_then(Value::as_str);
            if let Some(c) = color {
                check_color(c)?;
            }
            let text = s(args, "text")?.to_string();
            let n = store.create(color)?;
            Ok(full(&store.update(&n.id, |n| n.text = text)?))
        }
        "update_note" => {
            let id = s(args, "id")?;
            editable(store, id)?;
            let text = s(args, "text")?.to_string();
            Ok(full(&store.update(id, |n| n.text = text)?))
        }
        "append_note" => {
            let id = s(args, "id")?;
            editable(store, id)?;
            let add = s(args, "text")?.to_string();
            Ok(full(&store.update(id, |n| {
                if !n.text.is_empty() && !n.text.ends_with('\n') {
                    n.text.push('\n');
                }
                n.text.push_str(&add);
            })?))
        }
        "set_color" => {
            let id = s(args, "id")?;
            let c = s(args, "color")?.to_string();
            check_color(&c)?;
            editable(store, id)?;
            Ok(full(&store.update(id, |n| n.color = c)?))
        }
        "archive_note" => {
            let id = s(args, "id")?;
            Ok(full(&store.update(id, |n| n.archived = true)?))
        }
        "restore_note" => {
            let id = s(args, "id")?;
            Ok(full(&store.update(id, |n| n.archived = false)?))
        }
        _ => Err(format!("没有这个工具：{name}")),
    }
}
