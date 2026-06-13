//! xhh-cli: 小黑盒命令行工具
//!
//! 用法示例：
//! - `xhh login`            扫码登录
//! - `xhh info`             显示当前用户
//! - `xhh feeds`            列出帖子
//! - `xhh post -T 标题 -C 内容 -t 话题`  发帖
//! - `xhh comment <link_id> <text>`     发评论

use std::path::PathBuf;
use std::time::Duration;

use anyhow::{anyhow, Context, Result};
use clap::{Parser, Subcommand};
use qrcode::render::unicode;
use qrcode::QrCode;
use serde_json::Value;
use xhh_core::api::{
    comment as api_comment, feed as api_feed, interaction as api_inter, post as api_post,
    search as api_search, user as api_user,
};
use xhh_core::auth::QrPollResult;
use xhh_core::client::XhhClient;
use xhh_core::config::Config;

#[derive(Parser, Debug)]
#[command(
    name = "xhh",
    version,
    about = "小黑盒 Web API 命令行工具",
    long_about = "基于 xhh-core 实现的扫码登录、发帖、评论、点赞、收藏等全部功能"
)]
struct Cli {
    /// 自定义配置文件路径（默认使用平台规范路径）
    #[arg(long, global = true)]
    config: Option<PathBuf>,

    /// 启用 debug 日志
    #[arg(long, short = 'v', global = true)]
    verbose: bool,

    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand, Debug)]
enum Cmd {
    /// 扫码登录（强制覆盖现有凭据）
    Login,
    /// 退出登录（清空配置）
    Logout,
    /// 显示当前登录账号信息
    Info,
    /// 获取帖子列表
    Feeds {
        #[arg(long, default_value_t = 1)]
        page: u32,
        #[arg(long, default_value_t = 20)]
        limit: u32,
    },
    /// 查看帖子详情
    Detail { link_id: String },
    /// 我的帖子（当前用户动态）
    Mine { lastval: Option<String> },
    /// 发帖
    Post {
        #[arg(short = 'T', long)]
        title: String,
        #[arg(short = 'C', long)]
        content: String,
        #[arg(short = 't', long, help = "话题标签，多个用逗号分隔")]
        hashtag: Option<String>,
        #[arg(long, help = "社区名称（启用社区模式）")]
        community: Option<String>,
    },
    /// 删帖
    Delete { link_id: String },
    /// 发评论 / 回复评论
    Comment {
        link_id: String,
        text: String,
        #[arg(long, help = "回复的目标评论 ID")]
        reply_id: Option<String>,
        #[arg(long, help = "根评论 ID（回复子评论时填）")]
        root_id: Option<String>,
    },
    /// 切换帖子点赞
    LikePost { link_id: String },
    /// 切换评论点赞
    LikeComment { comment_id: String },
    /// 切换收藏
    Favour {
        link_id: String,
        #[arg(long)]
        folder_id: Option<String>,
    },
    /// 通用搜索
    Search {
        keyword: String,
        #[arg(short = 't', long, default_value = "综合")]
        search_type: String,
        #[arg(long, default_value_t = 10)]
        limit: u32,
    },
    /// 话题搜索
    Topic { keyword: String },
    /// 社区搜索
    Community { keyword: String },
    /// 用户主页
    Profile { userid: Option<String> },
    /// 通知列表
    Notifications {
        #[arg(long, default_value_t = 0)]
        offset: u32,
        #[arg(long, default_value_t = 10)]
        limit: u32,
    },
    /// Agent 子命令（LLM 驱动）
    ///
    /// 不带子命令直接进入交互式长对话模式。
    Agent {
        #[command(subcommand)]
        cmd: Option<AgentCmd>,
    },

    /// 启动本地 HTTP REST 服务（开发 API）
    Serve {
        /// 监听地址，如 127.0.0.1:9876 / :9876 / 9876
        #[arg(short = 'b', long, default_value = "127.0.0.1:9876")]
        bind: String,
        /// Bearer Token 鉴权（默认读 XHH_BEARER_TOKEN 环境变量）
        #[arg(short = 't', long)]
        token: Option<String>,
    },
}

#[derive(Subcommand, Debug)]
enum AgentCmd {
    /// 查看当前 Agent 配置
    Config,
    /// 设置 provider（openai / anthropic / ollama）
    SetProvider { name: String },
    /// 设置 OpenAI 兼容凭据（含 base_url）
    SetOpenai {
        api_key: String,
        #[arg(long, default_value = "gpt-4o-mini")]
        model: String,
        #[arg(long, default_value = "https://api.openai.com/v1")]
        base_url: String,
    },
    /// 设置 Anthropic Claude 凭据
    SetAnthropic {
        api_key: String,
        #[arg(long, default_value = "claude-haiku-4-5-20251001")]
        model: String,
        #[arg(long, default_value = "https://api.anthropic.com")]
        base_url: String,
        #[arg(long, default_value_t = 4096)]
        max_tokens: u32,
    },
    /// 设置 Ollama
    SetOllama {
        #[arg(long, default_value = "qwen2.5:14b")]
        model: String,
        #[arg(long, default_value = "http://localhost:11434")]
        base_url: String,
    },
    /// 设置每日调用上限与最大循环次数
    SetLimits {
        #[arg(long)]
        max_per_day: Option<u32>,
        #[arg(long)]
        max_loops: Option<u32>,
        #[arg(long, help = "试运行模式（不实际调用工具）")]
        dry_run: bool,
        #[arg(
            long,
            help = "启用配额计费（默认 false，用户自配置的 Provider 不消耗配额；仅后端 AI 服务需要时启用）"
        )]
        enforce_quota: bool,
        #[arg(long, help = "禁用 Agent 危险工具执行前确认")]
        no_confirm_dangerous_tools: bool,
    },
    /// 重置今日配额计数
    ResetQuota,
    /// 设置温度（0-2，留空表示用 provider 默认）
    SetTemperature { value: Option<f32> },
    /// 查看今日剩余配额
    Quota,
    /// 一键自动发帖（LLM 生成标题+正文+话题，然后调用 create_post）
    AutoPost {
        topic: String,
        #[arg(short = 't', long, help = "话题标签，逗号分隔")]
        hashtags: Option<String>,
    },
    /// 通用对话
    Chat { message: String },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    let filter = if cli.verbose {
        "debug"
    } else {
        "xhh_core=warn,info"
    };
    tracing_subscriber::fmt().with_env_filter(filter).init();

    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;

    rt.block_on(run(cli))
}

async fn run(cli: Cli) -> Result<()> {
    let cfg_path = cli.config.clone();

    match cli.cmd {
        Cmd::Login => {
            let existing = Config::load(cfg_path.as_deref()).ok();
            let cfg = rt_qr_login(existing).await?;
            println!(
                "\n登录成功！用户: {} (ID: {})\n凭据已保存至 {}",
                cfg.nickname,
                cfg.heybox_id,
                cfg_path
                    .clone()
                    .unwrap_or_else(Config::default_path)
                    .display()
            );
        }

        Cmd::Logout => {
            let empty = Config::default();
            empty.save(cfg_path.as_deref())?;
            println!("已退出登录");
        }

        Cmd::Info => {
            let cfg = require_config(&cfg_path)?;
            println!("  昵称:    {}", cfg.nickname);
            println!("  ID:      {}", cfg.heybox_id);
            println!("  设备:    {}", cfg.device_id);
            println!("  登录时间: {}", cfg.login_time_display());
            println!("  pkey:    {} 字符", cfg.pkey.len());
            println!("  cookie:  {} 字符", cfg.cookie.len());
            if cfg.cookie.is_empty() {
                println!("  [!] cookie 为空，请重新 `xhh login`");
            }
        }

        Cmd::Feeds { page, limit } => {
            let c = build_client(&cfg_path)?;
            let v = api_feed::feeds(
                &c,
                api_feed::FeedsQuery {
                    page: Some(page),
                    limit: Some(limit),
                    ..Default::default()
                },
            )
            .await?;
            print_feeds(&v);
        }

        Cmd::Detail { link_id } => {
            let c = build_client(&cfg_path)?;
            let v = api_feed::post_detail(&c, &link_id, Default::default()).await?;
            print_detail(&v);
        }

        Cmd::Mine { lastval } => {
            let c = build_client(&cfg_path)?;
            let v = api_feed::user_events(&c, None, lastval.as_deref()).await?;
            print_user_events(&v);
        }

        Cmd::Post {
            title,
            content,
            hashtag,
            community,
        } => {
            let c = build_client(&cfg_path)?;
            let hashtags: Vec<String> = hashtag
                .map(|s| {
                    s.split(',')
                        .map(|x| x.trim().to_string())
                        .filter(|x| !x.is_empty())
                        .collect()
                })
                .unwrap_or_default();

            // 社区模式：搜索社区，拿到 topic_id，link_tag=27
            let (topic_ids, link_tag) = if let Some(comm) = community {
                let sr = api_search::search_community(&c, &comm).await?;
                let first = sr
                    .pointer("/result/search_result")
                    .and_then(|r| r.as_array())
                    .and_then(|arr| arr.first())
                    .ok_or_else(|| anyhow!("未找到社区 {}", comm))?;
                let tid = first
                    .get("topic_id")
                    .and_then(|v| v.as_i64())
                    .ok_or_else(|| anyhow!("社区响应缺 topic_id"))?;
                (vec![tid.to_string()], 27i64)
            } else {
                (vec!["58144".into()], 28i64)
            };

            let req = api_post::CreatePostReq {
                title,
                content: content.into(),
                topic_ids,
                hashtags,
                link_tag,
                ..Default::default()
            };
            let v = api_post::create_post(&c, req).await?;
            let link_id = v
                .pointer("/result/link_id")
                .or_else(|| v.get("link_id"))
                .map(|i| i.to_string())
                .unwrap_or_else(|| v.to_string());
            println!("发帖完成。返回: {}", link_id);
        }

        Cmd::Delete { link_id } => {
            let c = build_client(&cfg_path)?;
            let v = api_post::delete_post(&c, &link_id).await?;
            println!(
                "删帖完成: {}",
                v.get("status").and_then(|s| s.as_str()).unwrap_or("?")
            );
        }

        Cmd::Comment {
            link_id,
            text,
            reply_id,
            root_id,
        } => {
            let c = build_client(&cfg_path)?;
            let req = if let Some(rid) = reply_id.clone() {
                api_comment::CreateCommentReq::reply(
                    &link_id,
                    text,
                    &rid,
                    root_id.as_deref().unwrap_or(&rid),
                )
            } else {
                api_comment::CreateCommentReq::top_level(&link_id, text)
            };
            let v = api_comment::create_comment(&c, req).await?;
            let cid = v
                .pointer("/result/comment_id")
                .map(|i| i.to_string())
                .unwrap_or_else(|| v.to_string());
            println!("评论完成。comment_id: {}", cid);
        }

        Cmd::LikePost { link_id } => {
            let c = build_client(&cfg_path)?;
            let v = api_inter::like_post(&c, &link_id, 1).await?;
            println!(
                "帖子点赞: {}",
                v.get("status").and_then(|s| s.as_str()).unwrap_or("?")
            );
        }

        Cmd::LikeComment { comment_id } => {
            let c = build_client(&cfg_path)?;
            let v = api_inter::toggle_like_comment(&c, &comment_id).await?;
            println!(
                "评论点赞切换: {}",
                v.get("status").and_then(|s| s.as_str()).unwrap_or("?")
            );
        }

        Cmd::Favour { link_id, folder_id } => {
            let c = build_client(&cfg_path)?;
            let v = api_inter::toggle_favourite(&c, &link_id, folder_id.as_deref()).await?;
            println!(
                "收藏切换: {}",
                v.get("status").and_then(|s| s.as_str()).unwrap_or("?")
            );
        }

        Cmd::Search {
            keyword,
            search_type,
            limit,
        } => {
            let c = build_client(&cfg_path)?;
            let st = parse_search_type(&search_type)?;
            let v = api_search::search(
                &c,
                api_search::SearchReq {
                    q: keyword,
                    search_type: st,
                    offset: 0,
                    limit,
                    ..Default::default()
                },
            )
            .await?;
            print_search(&v);
        }

        Cmd::Topic { keyword } => {
            let c = build_client(&cfg_path)?;
            let v = api_search::search_topic(&c, &keyword).await?;
            print_topic_search(&v);
        }

        Cmd::Community { keyword } => {
            let c = build_client(&cfg_path)?;
            let v = api_search::search_community(&c, &keyword).await?;
            print_community_search(&v);
        }

        Cmd::Profile { userid } => {
            let c = build_client(&cfg_path)?;
            let v = api_user::user_profile(&c, userid.as_deref()).await?;
            print_profile(&v);
        }

        Cmd::Notifications { offset, limit } => {
            let c = build_client(&cfg_path)?;
            let v = xhh_core::api::notification::list_all_messages(&c, offset, limit).await?;
            print_notifications(&v);
        }

        Cmd::Agent { cmd } => match cmd {
            Some(agent_cmd) => handle_agent(agent_cmd, &cfg_path).await?,
            None => agent_repl(&cfg_path).await?,
        },

        Cmd::Serve { bind, token } => {
            let addr = xhh_http::parse_addr(&bind).map_err(|e| anyhow!("地址解析失败: {}", e))?;
            let token = token
                .or_else(|| std::env::var("XHH_BEARER_TOKEN").ok())
                .filter(|s| !s.is_empty());
            let state = xhh_http::AppState::new(token.clone());
            let app = xhh_http::build_app(state);
            let listener = tokio::net::TcpListener::bind(addr).await?;
            let hint = if token.is_some() {
                "（已启用 Bearer Token 鉴权）"
            } else {
                "（未启用鉴权，本地可访问）"
            };
            println!("xhh serve 启动 → http://{} {}", addr, hint);
            println!("Swagger UI: http://{}/docs", addr);
            println!("按 Ctrl+C 退出\n");
            axum::serve(listener, app).await?;
        }
    }

    Ok(())
}

// ─── Agent 命令处理 ──────────────────────────────────────

async fn agent_repl(cfg_path: &Option<PathBuf>) -> Result<()> {
    use xhh_agent::config::{AgentConfig, DailyCounters};
    use xhh_agent::prompt;
    use xhh_agent::provider::ChatMessage;
    use xhh_agent::runner::AgentRunner;

    let cfg = Config::load(cfg_path.as_deref()).context("读取配置失败")?;
    if !cfg.has_credentials() {
        return Err(anyhow!("未登录，请先 xhh login"));
    }
    let ac = AgentConfig::load(None).context("读取 agent 配置失败")?;
    let client = XhhClient::new(cfg).context("构建 HTTP 客户端失败")?;
    let counters = DailyCounters::load(None)?;
    let mut runner = AgentRunner::from_config(ac, counters, client)
        .context("构建 Agent 失败")?
        .with_confirmation_handler(Box::new(xhh_agent::runner::StdinConfirmationHandler));

    // 持久化消息上下文（REPL 多轮对话核心）
    let mut messages: Vec<ChatMessage> = vec![ChatMessage::system(prompt::SYSTEM_PROMPT)];

    println!("小黑盒 Agent 交互模式");
    println!("输入消息开始对话，输入 exit 或 quit 退出\n");

    loop {
        print!("  > ");
        use std::io::Write;
        std::io::stdout().flush().ok();

        let mut input = String::new();
        match std::io::stdin().read_line(&mut input) {
            Ok(0) => break, // EOF
            Ok(_) => {}
            Err(e) => {
                tracing::error!(error = %e, "读取输入失败");
                break;
            }
        }
        let input = input.trim();
        if input.is_empty() {
            continue;
        }
        if matches!(input, "exit" | "quit" | "q" | "退出") {
            println!("再见！");
            break;
        }

        match runner.chat_with_history(&mut messages, input).await {
            Ok(result) => {
                println!();
                print_agent_result(&result);
            }
            Err(e) => {
                tracing::error!(error = %e, "Agent 错误");
            }
        }
    }
    Ok(())
}

async fn handle_agent(cmd: AgentCmd, _cfg_path: &Option<PathBuf>) -> Result<()> {
    use xhh_agent::config::{AgentConfig, AnthropicCfg, OllamaCfg, OpenAiCfg};
    use xhh_agent::runner::AgentRunner;

    let mut ac = AgentConfig::load(None).context("读取 agent 配置失败")?;

    match cmd {
        AgentCmd::Config => {
            let json = serde_json::to_string_pretty(&ac)?;
            println!("{}", json);
        }

        AgentCmd::SetProvider { name } => {
            if !["openai", "anthropic", "claude", "ollama"].contains(&name.as_str()) {
                return Err(anyhow!(
                    "未知 provider: {}（支持 openai/anthropic/ollama）",
                    name
                ));
            }
            ac.active_provider = if name == "claude" {
                "anthropic".into()
            } else {
                name
            };
            ac.save(None)?;
            println!("已切换到 provider: {}", ac.active_provider);
        }

        AgentCmd::SetOpenai {
            api_key,
            model,
            base_url,
        } => {
            ac.openai = Some(OpenAiCfg {
                api_key,
                model,
                base_url,
            });
            ac.active_provider = "openai".into();
            ac.save(None)?;
            println!("OpenAI 配置已保存");
        }

        AgentCmd::SetAnthropic {
            api_key,
            model,
            base_url,
            max_tokens,
        } => {
            ac.anthropic = Some(AnthropicCfg {
                api_key,
                model,
                base_url,
                max_tokens,
            });
            ac.active_provider = "anthropic".into();
            ac.save(None)?;
            println!("Anthropic 配置已保存");
        }

        AgentCmd::SetOllama { model, base_url } => {
            ac.ollama = Some(OllamaCfg { model, base_url });
            ac.active_provider = "ollama".into();
            ac.save(None)?;
            println!("Ollama 配置已保存");
        }

        AgentCmd::SetLimits {
            max_per_day,
            max_loops,
            dry_run,
            enforce_quota,
            no_confirm_dangerous_tools,
        } => {
            if let Some(m) = max_per_day {
                ac.max_per_day = m;
            }
            if let Some(m) = max_loops {
                ac.max_loops = m;
            }
            ac.dry_run = dry_run;
            ac.quota_enforced = enforce_quota;
            ac.confirm_dangerous_tools = !no_confirm_dangerous_tools;
            ac.save(None)?;
            println!(
                "已保存：max_per_day={}, max_loops={}, dry_run={}, quota_enforced={}, confirm_dangerous_tools={}",
                ac.max_per_day, ac.max_loops, ac.dry_run, ac.quota_enforced, ac.confirm_dangerous_tools
            );
        }

        AgentCmd::ResetQuota => {
            let mut counters = xhh_agent::config::DailyCounters::load(None)?;
            counters.count = 0;
            counters.save(None)?;
            println!("已重置今日配额计数（当前已用 0 次）");
        }

        AgentCmd::SetTemperature { value } => {
            ac.temperature = value;
            ac.save(None)?;
            println!("温度已保存为: {:?}", ac.temperature);
        }

        AgentCmd::Quota => {
            if !ac.quota_enforced {
                println!(
                    "当前 Provider: {}（用户自配置）— 不消耗配额，调用次数无限制",
                    if ac.active_provider.is_empty() {
                        "(未设置)"
                    } else {
                        &ac.active_provider
                    }
                );
                println!("（如需启用配额：xhh agent set-limits --enforce-quota）");
            } else {
                let counters = xhh_agent::config::DailyCounters::load(None)?;
                let remaining = counters.remaining(ac.max_per_day);
                let used = ac.max_per_day.saturating_sub(remaining);
                println!(
                    "当前 Provider: {}（后端 AI 服务，已启用配额）",
                    ac.active_provider
                );
                println!(
                    "今日配额: 已用 {} / {}，剩余 {}",
                    used, ac.max_per_day, remaining
                );
                println!("（重置：xhh agent reset-quota）");
            }
        }

        AgentCmd::AutoPost { topic, hashtags } => {
            let cfg = Config::load(_cfg_path.as_deref()).context("读取配置失败")?;
            if !cfg.has_credentials() {
                return Err(anyhow!("未登录，请先 xhh login"));
            }
            let client = XhhClient::new(cfg).context("构建 HTTP 客户端失败")?;
            let counters = xhh_agent::config::DailyCounters::load(None)?;
            let mut runner = AgentRunner::from_config(ac, counters, client)
                .context("构建 Agent 失败")?
                .with_confirmation_handler(Box::new(xhh_agent::runner::StdinConfirmationHandler));
            let tags: Vec<String> = hashtags
                .map(|s| {
                    s.split(',')
                        .map(|x| x.trim().to_string())
                        .filter(|x| !x.is_empty())
                        .collect()
                })
                .unwrap_or_default();
            println!("\n收到指令: {}", topic);
            if !tags.is_empty() {
                println!("提示话题: {}\n", tags.join(", "));
            } else {
                println!();
            }
            let result = runner
                .auto_post(&topic, &tags)
                .await
                .context("Agent 执行失败")?;
            print_agent_result(&result);
        }

        AgentCmd::Chat { message } => {
            let cfg = Config::load(_cfg_path.as_deref()).context("读取配置失败")?;
            if !cfg.has_credentials() {
                return Err(anyhow!("未登录，请先 xhh login"));
            }
            let client = XhhClient::new(cfg).context("构建 HTTP 客户端失败")?;
            let counters = xhh_agent::config::DailyCounters::load(None)?;
            let mut runner = AgentRunner::from_config(ac, counters, client)
                .context("构建 Agent 失败")?
                .with_confirmation_handler(Box::new(xhh_agent::runner::StdinConfirmationHandler));
            println!("\nAgent 启动...\n");
            let result = runner.chat(&message).await.context("Agent 执行失败")?;
            print_agent_result(&result);
        }
    }
    Ok(())
}

fn print_agent_result(r: &xhh_agent::runner::AgentResult) {
    println!("──────────────────────────────────────");
    if !r.final_output.is_empty() {
        println!("最终输出:\n{}\n", r.final_output);
    }
    if !r.tool_calls.is_empty() {
        println!("工具调用（{} 次）:", r.tool_calls.len());
        for (i, name) in r.tool_calls.iter().enumerate() {
            println!("  {}. {}", i + 1, name);
        }
    }
    println!("消耗轮数: {}", r.loops_used);
    println!("日志:");
    for log in &r.logs {
        println!("  {}", log.message);
    }
    println!("──────────────────────────────────────");
}

// ─── 辅助函数 ─────────────────────────────────────────────

fn require_config(cfg_path: &Option<PathBuf>) -> Result<Config> {
    let cfg = Config::load(cfg_path.as_deref()).context("读取配置失败")?;
    if !cfg.has_credentials() {
        return Err(anyhow!("未登录，请先执行 `xhh login`"));
    }
    Ok(cfg)
}

fn build_client(cfg_path: &Option<PathBuf>) -> Result<XhhClient> {
    let cfg = require_config(cfg_path)?;
    XhhClient::new(cfg).context("构建 HTTP 客户端失败")
}

fn parse_search_type(s: &str) -> Result<api_search::SearchType> {
    use api_search::SearchType::*;
    Ok(match s {
        "综合" | "default" => Comprehensive,
        "内容" | "content" => Content,
        "游戏" | "game" => Game,
        "小程序" | "mini" => MiniProgram,
        "用户" | "user" => User,
        "话题" | "topic" => Topic,
        "商品" | "product" => Product,
        other => {
            return Err(anyhow!(
                "未知 search_type: {}（支持 综合/内容/游戏/小程序/用户/话题/商品）",
                other
            ))
        }
    })
}

/// 阻塞版扫码登录（终端显示 unicode 二维码）
async fn rt_qr_login(existing: Option<Config>) -> Result<Config> {
    use xhh_core::auth;

    let device_id = existing
        .as_ref()
        .map(|c| c.device_id.clone())
        .unwrap_or_default();
    let anon_client = xhh_core::client::XhhClient::anonymous(Some(device_id))?;

    println!("\n小黑盒扫码登录\n");

    let qr = auth::get_qr_code(&anon_client).await?;

    // 用 unicode Dense1x2 渲染（半块字符）
    let code = QrCode::new(qr.qr_url.as_bytes()).map_err(|e| anyhow!("QR 生成失败: {}", e))?;
    let image = code
        .render::<unicode::Dense1x2>()
        .dark_color(unicode::Dense1x2::Light)
        .light_color(unicode::Dense1x2::Dark)
        .build();
    println!("{}", image);
    println!("  也可以手动打开链接扫码: {}\n", qr.qr_url);

    let deadline = std::time::Instant::now() + Duration::from_secs(qr.expire);
    let mut last_log = std::time::Instant::now();
    loop {
        if std::time::Instant::now() > deadline {
            return Err(anyhow!("扫码超时"));
        }
        match auth::poll_qr_state_once(&anon_client, &qr.raw_query, &anon_client.device_id).await? {
            QrPollResult::Waiting { msg } => {
                if last_log.elapsed() > Duration::from_millis(1500) {
                    println!("  等待扫码... [{}]", msg);
                    last_log = std::time::Instant::now();
                }
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
            QrPollResult::Scanned => {
                println!("  已扫码，请在 APP 内确认登录");
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
            QrPollResult::Success(mut success) => {
                success.config.device_id = anon_client.device_id.clone();
                success.config.save(None)?;
                return Ok(success.config);
            }
        }
    }
}

// ─── 打印辅助 ─────────────────────────────────────────────

fn print_feeds(v: &Value) {
    let links = match v.pointer("/result/links").and_then(|l| l.as_array()) {
        Some(a) => a,
        None => {
            println!("获取 feeds 失败: {}", v);
            return;
        }
    };
    if links.is_empty() {
        println!("（暂无帖子）");
        return;
    }
    println!("\nFeeds 共 {} 条：\n", links.len());
    for l in links {
        let id = l
            .get("linkid")
            .map(|v| v.to_string())
            .unwrap_or_else(|| "?".into());
        let title = l
            .get("title")
            .and_then(|v| v.as_str())
            .unwrap_or("(无标题)");
        let author = l
            .pointer("/user/username")
            .and_then(|v| v.as_str())
            .unwrap_or("?");
        let comments = l.get("comment_num").and_then(|v| v.as_i64()).unwrap_or(0);
        let topic = l
            .pointer("/topics/0/name")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let topic_tag = if topic.is_empty() {
            String::new()
        } else {
            format!(" [{}]", topic)
        };
        println!(
            "  [{}] {} — {} ({}评){}",
            id, title, author, comments, topic_tag
        );
    }
}

fn print_detail(v: &Value) {
    let link = match v.pointer("/result/link") {
        Some(l) => l,
        None => {
            println!("获取详情失败: {}", v);
            return;
        }
    };
    let title = link
        .get("title")
        .and_then(|v| v.as_str())
        .unwrap_or("(无标题)");
    let author = link
        .pointer("/user/username")
        .and_then(|v| v.as_str())
        .unwrap_or("?");
    let text = link.get("text").and_then(|v| v.as_str()).unwrap_or("");
    println!("\n  标题: {}", title);
    println!("  作者: {}", author);
    if !text.is_empty() {
        let preview = if text.chars().count() > 200 {
            text.chars().take(200).collect::<String>() + "..."
        } else {
            text.to_string()
        };
        println!("  正文: {}", preview);
    }
}

fn print_user_events(v: &Value) {
    let moments = match v.pointer("/result/moments").and_then(|l| l.as_array()) {
        Some(a) => a,
        None => {
            println!("获取动态失败: {}", v);
            return;
        }
    };
    if moments.is_empty() {
        println!("（暂无帖子）");
        return;
    }
    println!("\n共 {} 条动态：\n", moments.len());
    for m in moments {
        let id = m
            .get("linkid")
            .map(|v| v.to_string())
            .unwrap_or_else(|| "?".into());
        let title = m
            .get("title")
            .and_then(|v| v.as_str())
            .unwrap_or("(无标题)");
        let comments = m.get("comment_num").and_then(|v| v.as_i64()).unwrap_or(0);
        let ups = m.get("up").and_then(|v| v.as_i64()).unwrap_or(0);
        let ts = m.get("create_at").and_then(|v| v.as_i64()).unwrap_or(0);
        let time_str = if ts > 0 {
            chrono::DateTime::from_timestamp(ts, 0)
                .map(|t| t.format("%Y-%m-%d %H:%M").to_string())
                .unwrap_or_else(|| "?".into())
        } else {
            "?".into()
        };
        println!(
            "  [{}] {} ({}评, {}赞 @ {})",
            id, title, comments, ups, time_str
        );
    }
}

fn print_search(v: &Value) {
    let items = match v.pointer("/result/items").and_then(|l| l.as_array()) {
        Some(a) => a,
        None => {
            println!("搜索失败: {}", v);
            return;
        }
    };
    if items.is_empty() {
        println!("（无搜索结果）");
        return;
    }
    println!("\n搜索结果 {} 条：\n", items.len());
    for item in items {
        let t = item.get("type").and_then(|v| v.as_str()).unwrap_or("?");
        let info = item.get("info").cloned().unwrap_or(Value::Null);
        match t {
            "user" => {
                let name = info.get("username").and_then(|v| v.as_str()).unwrap_or("?");
                let uid = info.get("userid").and_then(|v| v.as_str()).unwrap_or("?");
                println!("  [用户] {} ({})", name, uid);
            }
            "内容" | "content" => {
                let title = info
                    .get("title")
                    .and_then(|v| v.as_str())
                    .unwrap_or("(无标题)");
                let lid = info
                    .get("linkid")
                    .map(|v| v.to_string())
                    .unwrap_or_else(|| "?".into());
                println!("  [帖子] {} ({})", title, lid);
            }
            "topic" => {
                let name = info.get("name").and_then(|v| v.as_str()).unwrap_or("?");
                let tid = info
                    .get("topic_id")
                    .map(|v| v.to_string())
                    .unwrap_or_else(|| "?".into());
                println!("  [社区] {} ({})", name, tid);
            }
            "game" => {
                let name = info.get("name").and_then(|v| v.as_str()).unwrap_or("?");
                println!("  [游戏] {}", name);
            }
            "space" => {}
            other => println!("  [{}] {}", other, info),
        }
    }
}

fn print_topic_search(v: &Value) {
    let arr = match v
        .pointer("/result/search_result")
        .and_then(|l| l.as_array())
    {
        Some(a) => a,
        None => {
            println!("话题搜索失败: {}", v);
            return;
        }
    };
    for t in arr {
        let name = t.get("name").and_then(|v| v.as_str()).unwrap_or("?");
        let id = t
            .get("id")
            .map(|v| v.to_string())
            .unwrap_or_else(|| "?".into());
        let num = t
            .pointer("/num/content_num")
            .and_then(|v| v.as_i64())
            .unwrap_or(0);
        println!("  [{}] {} ({}篇讨论)", id, name, num);
    }
}

fn print_community_search(v: &Value) {
    let arr = match v
        .pointer("/result/search_result")
        .and_then(|l| l.as_array())
    {
        Some(a) => a,
        None => {
            println!("社区搜索失败: {}", v);
            return;
        }
    };
    for c in arr {
        let name = c.get("name").and_then(|v| v.as_str()).unwrap_or("?");
        let tid = c
            .get("topic_id")
            .map(|v| v.to_string())
            .unwrap_or_else(|| "?".into());
        let hot = c
            .pointer("/hot/desc")
            .and_then(|v| v.as_str())
            .unwrap_or("0");
        println!("  [{}] {} (热度 {})", tid, name, hot);
    }
}

fn print_profile(v: &Value) {
    let detail = match v.pointer("/result/account_detail") {
        Some(d) => d,
        None => {
            println!("获取主页失败: {}", v);
            return;
        }
    };
    let name = detail
        .get("username")
        .and_then(|v| v.as_str())
        .unwrap_or("?");
    let uid = detail
        .get("userid")
        .map(|v| v.to_string())
        .unwrap_or_else(|| "?".into());
    let sig = detail
        .get("signature")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let ip = detail
        .get("ip_location")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let level = detail
        .pointer("/level_info/level")
        .and_then(|v| v.as_i64())
        .unwrap_or(0);
    let follow = detail
        .pointer("/bbs_info/follow_num")
        .and_then(|v| v.as_i64())
        .unwrap_or(0);
    let fans = detail
        .pointer("/bbs_info/fan_num")
        .and_then(|v| v.as_i64())
        .unwrap_or(0);
    let posts = detail
        .pointer("/bbs_info/post_link_num")
        .and_then(|v| v.as_i64())
        .unwrap_or(0);

    println!("\n  {} (ID: {}, Lv{})", name, uid, level);
    if !sig.is_empty() {
        println!("  签名: {}", sig);
    }
    if !ip.is_empty() {
        println!("  IP:   {}", ip);
    }
    println!("  关注: {}  粉丝: {}  帖子: {}\n", follow, fans, posts);
}

fn print_notifications(v: &Value) {
    let msgs = match v.pointer("/result/messages").and_then(|l| l.as_array()) {
        Some(a) => a,
        None => {
            println!("获取通知失败: {}", v);
            return;
        }
    };
    if msgs.is_empty() {
        println!("（无通知）");
        return;
    }
    println!("\n通知 {} 条：\n", msgs.len());
    for m in msgs {
        let user = m.get("username").and_then(|v| v.as_str()).unwrap_or("?");
        let text = m.get("comment_text").and_then(|v| v.as_str()).unwrap_or("");
        let link = m
            .get("link_id")
            .map(|v| v.to_string())
            .unwrap_or_else(|| "?".into());
        let ts = m.get("create_at").and_then(|v| v.as_i64()).unwrap_or(0);
        let time_str = if ts > 0 {
            chrono::DateTime::from_timestamp(ts, 0)
                .map(|t| t.format("%Y-%m-%d %H:%M").to_string())
                .unwrap_or_else(|| "?".into())
        } else {
            "?".into()
        };
        println!("  [{}] {} @帖子 {}: {}", time_str, user, link, text);
    }
}
