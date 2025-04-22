use thiserror::Error;

#[derive(Debug, Error)]
pub enum TranslatorError {
    #[error("翻译服务不可用")]
    NoAvailableBackend,
    
    #[error("翻译失败: {0}")]
    TranslationFailed(#[from] TranslationError),
    
    #[error("配置错误: {0}")]
    ConfigError(String),
    
    #[error("系统错误: {0}")]
    IoError(#[from] std::io::Error),
    
    #[error("配置解析错误: {0}")]
    SerdeError(#[from] serde_json::Error),
}

#[derive(Debug, Error)]
pub enum TranslationError {
    #[error("网络连接失败: {0}")]
    NetworkError(String),
    
    #[error("认证失败: {0}")]
    AuthenticationError(String),
    
    #[error("无效的响应: {0}")]
    InvalidResponse(String),
    
    #[error("服务错误: {0}")]
    ServiceError(String),
    
    #[error("超出使用限制")]
    RateLimitExceeded,
}

pub fn format_error_display(error: &anyhow::Error) -> (String, String, &'static str) {
    // 返回 (用户友好消息, 技术细节, 错误类型CSS类)
    let user_msg = match error.downcast_ref::<TranslatorError>() {
        Some(e) => match e {
            TranslatorError::NoAvailableBackend => 
                "无可用的翻译后端服务。请在配置文件中设置翻译服务。".to_string(),
            TranslatorError::TranslationFailed(te) => match te {
                TranslationError::NetworkError(_) => 
                    "网络连接失败，请检查网络连接后重试。".to_string(),
                TranslationError::AuthenticationError(_) => 
                    "翻译服务认证失败，请检查配置文件中的密钥信息。".to_string(),
                TranslationError::InvalidResponse(_) => 
                    "翻译服务返回异常，请稍后重试。".to_string(),
                TranslationError::ServiceError(_) => 
                    "翻译服务暂时不可用，请稍后重试。".to_string(),
                TranslationError::RateLimitExceeded => 
                    "超出翻译服务使用限制，请稍后重试。".to_string(),
            },
            TranslatorError::ConfigError(_) => 
                "配置文件读取失败，请检查配置文件格式是否正确。".to_string(),
            TranslatorError::IoError(_) => 
                "系统文件操作失败，请检查文件权限。".to_string(),
            TranslatorError::SerdeError(_) => 
                "配置文件格式错误，请检查JSON格式是否正确。".to_string(),
        },
        None => "发生未知错误。".to_string(),
    };

    let error_class = if error.downcast_ref::<TranslatorError>()
        .and_then(|e| match e {
            TranslatorError::TranslationFailed(te) => Some(te),
            _ => None,
        })
        .map(|te| matches!(te, TranslationError::NetworkError(_)))
        .unwrap_or(false)
    {
        "network"
    } else if error.is::<TranslatorError>() {
        "config"
    } else {
        "service"
    };

    // 获取完整的错误链信息
    let technical_details = format!("{:#}", error);

    (user_msg, technical_details, error_class)
}
