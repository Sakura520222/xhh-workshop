//! xhh-app Tauri 后端入口

pub mod commands;
pub mod state;

use state::AppState;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .manage(AppState::try_load())
        .setup(|app| {
            #[cfg(target_os = "windows")]
            {
                use window_vibrancy::{apply_acrylic, apply_mica};
                if let Some(window) = app.get_webview_window("main") {
                    let _ = apply_mica(&window, Some(true))
                        .or_else(|_| apply_acrylic(&window, Some((18, 18, 18, 125))));
                }
            }

            // 设置窗口图标（任务栏等）
            if let Some(window) = app.get_webview_window("main") {
                let img_bytes = include_bytes!("../icons/32x32.png");
                if let Ok(img) = image::load_from_memory(img_bytes) {
                    let rgba = img.to_rgba8();
                    let icon =
                        tauri::image::Image::new_owned(rgba.into_raw(), img.width(), img.height());
                    let _ = window.set_icon(icon);
                }
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // auth
            commands::auth_get_qr_code,
            commands::auth_login,
            commands::auth_status,
            commands::auth_logout,
            // feeds
            commands::feeds_list,
            commands::post_detail,
            commands::community_feeds,
            // post
            commands::post_create,
            commands::post_delete,
            // comment
            commands::comment_create,
            commands::comment_list,
            commands::sub_comments,
            // interaction
            commands::like_post,
            commands::like_comment,
            commands::favourite,
            // search / user
            commands::search,
            commands::search_community,
            commands::user_profile,
            // agent
            commands::agent_chat,
            commands::agent_chat_stream,
            commands::agent_history_get,
            commands::agent_history_save,
            commands::agent_history_clear,
            commands::agent_auto_post,
            commands::agent_reset,
            commands::agent_get_config,
            commands::agent_save_config,
            // ai
            commands::ai_analyze_stream,
            commands::ai_cache_get,
            commands::ai_cache_save,
            // image
            commands::save_image,
            commands::upload_image,
            // topic
            commands::search_topic,
            // emoji
            commands::emoji_list,
            // notifications
            commands::notifications,
            // favourites
            commands::favour_folders,
            commands::favour_folder,
            // follow / user
            commands::follow_user,
            commands::unfollow_user,
            commands::following_list,
            commands::follower_list,
            commands::user_events,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
