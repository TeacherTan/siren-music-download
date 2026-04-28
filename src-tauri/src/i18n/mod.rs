//! 后端 Fluent 翻译基础设施。
//!
//! 基于 `fluent-templates` 提供编译期嵌入的多语言文案加载与查询入口，
//! 供系统通知、偏好校验、用户可见错误等后端场景按当前 `Locale` 输出本地化文案。

use std::borrow::Cow;
use std::collections::HashMap;

use fluent_templates::fluent_bundle::FluentValue;
use fluent_templates::{static_loader, Loader};
use unic_langid::LanguageIdentifier;

use crate::preferences::Locale;

static_loader! {
    static LOCALES = {
        locales: "./locales",
        fallback_language: "zh-CN"
    };
}

fn lang_id(locale: Locale) -> LanguageIdentifier {
    match locale {
        Locale::ZhCN => "zh-CN".parse().unwrap(),
        Locale::EnUS => "en-US".parse().unwrap(),
    }
}

/// 按当前语言查询简单文案。
///
/// 目标语言缺 key 时回退 `zh-CN`；仍缺失时返回 key 本身。
pub fn tr(locale: Locale, key: &str) -> String {
    LOCALES
        .try_lookup(&lang_id(locale), key)
        .unwrap_or_else(|| key.to_string())
}

/// 按当前语言查询带参数的文案。
///
/// 参数通过 `args` 传入，对应 `.ftl` 文件中的 `{ $param }` 占位符。
/// 目标语言缺 key 时回退 `zh-CN`；仍缺失时返回 key 本身。
pub fn tr_args(
    locale: Locale,
    key: &str,
    args: &HashMap<Cow<'static, str>, FluentValue>,
) -> String {
    LOCALES
        .try_lookup_with_args(&lang_id(locale), key, args)
        .unwrap_or_else(|| key.to_string())
}

/// 构建 Fluent 参数映射的便捷宏。
///
/// 用法: `fluent_args!("count" => 3, "name" => "test")`
macro_rules! fluent_args {
    ($($key:expr => $val:expr),* $(,)?) => {{
        let mut args = std::collections::HashMap::new();
        $(
            args.insert(
                std::borrow::Cow::Borrowed($key),
                fluent_templates::fluent_bundle::FluentValue::from($val),
            );
        )*
        args
    }};
}

pub(crate) use fluent_args;
