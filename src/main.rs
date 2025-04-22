use std::env;
use anyhow::Context;
mod translation;
use translation::{
    Config, TranslationManager, format_error_display
};

const CSS: &str = r#"<style type="text/css">
.engine {
    font-family: "MiSansVF";
    font-size: 18px;
    color: #578bc5;
}
.originalText {
    font-size: 120%;
    font-family: "MiSansVF";
    font-weight: 600;
    display: inline-block;
    margin: 0rem 0rem 0rem 0rem;
    color: #2a5598;
    margin-bottom: 0.6rem;
}
.frame {
    margin: 1rem 0.5rem 0.5rem 0;
    padding: 0.7rem 0.5rem 0.5rem 0;
    border-top: 3px dashed #eaeef6;
}
.error-container {
    margin: 1rem 0;
    font-family: "MiSansVF";
}
.error-message {
    padding: 0.5em;
    font-weight: 500;
    font-size: 16px;
    border-radius: 4px;
    margin-bottom: 0.5em;
}
.error-details {
    font-family: monospace;
    font-size: 14px;
    padding: 0.5em;
    background: #f8f9fa;
    border-radius: 4px;
    white-space: pre-wrap;
    display: none;
}
.error-container:hover .error-details {
    display: block;
}
.error-message.network {
    color: #e67e22;
    background-color: #fef5ea;
}
.error-message.config {
    color: #c0392b;
    background-color: #fae9e8;
}
.error-message.service {
    color: #2980b9;
    background-color: #eaf2fa;
}
definition {
    font-family: "MiSansVF";
    color: #2a5598;
    height: 120px;
    padding: 0.05em;
    font-weight: 500;
    font-size: 16px;
}
</style>"#;

fn count_languages(text: &str) -> (usize, usize) {
    let mut chinese_count = 0;
    let mut english_count = 0;

    for ch in text.chars() {
        if ch.is_ascii() {
            english_count += 1;
        } else if is_chinese(ch) {
            chinese_count += 1;
        }
    }

    (chinese_count, english_count)
}

fn is_chinese(ch: char) -> bool {
    match ch {
        '\u{4E00}'..='\u{9FFF}' |
        '\u{3400}'..='\u{4DBF}' |
        '\u{20000}'..='\u{2A6DF}' |
        '\u{2A700}'..='\u{2B73F}' |
        '\u{2B740}'..='\u{2B81F}' |
        '\u{2B820}'..='\u{2CEAF}' |
        '\u{F900}'..='\u{FAFF}' |
        '\u{2F800}'..='\u{2FA1F}' => true,
        _ => false,
    }
}

fn display_error(error: &anyhow::Error) {
    // 记录详细错误到stderr
    eprintln!("Error occurred: {:#}", error);
    
    // 获取用户友好的错误信息和详细信息
    let (user_msg, technical_details, error_class) = format_error_display(error);
    
    // 输出格式化的错误信息
    println!(r#"<div class="error-container">
    <div class="error-message {}">{}</div>
    <div class="error-details">{}</div>
</div>"#, error_class, user_msg, technical_details);
}

fn main() -> anyhow::Result<()> {
    let args = env::args().collect::<Vec<String>>();

    if args.len() < 2 {
        return Err(anyhow::anyhow!("Usage: {} <text>", args[0]));
    }

    // Load configuration
    let config = Config::load()
        .context("Failed to load configuration")?;

    // Initialize translation manager
    let mut manager = TranslationManager::new();

    // Initialize backends from config
    for (_name, backend_config) in config.backends {
        manager.add_backend(backend_config.as_backend());
    }

    let (chinese_count, english_count) = count_languages(&args[1]);
    let target_lang = if chinese_count > english_count { "en" } else { "zh" };

    // Print CSS and original text
    println!("{}", CSS);
    println!("<div class=\"originalText\">{}</div>", args[1]);
    println!("<br><br>");

    // Perform translation if a default backend is configured
    if let Some(backend_name) = &config.default_backend {
        match manager.translate_with(backend_name, &args[1], "auto", target_lang) {
            Ok(translated_text) => {
                println!("<div class=\"frame\">");
                println!("<definition>{}</definition>", translated_text);
                println!("</div>");
            }
            Err(e) => {
                display_error(&e);
            }
        }
    } else {
        display_error(&anyhow::anyhow!("No default translation backend configured. Please set one in config.json."));
    }

    println!("<br>");
    Ok(())
}
