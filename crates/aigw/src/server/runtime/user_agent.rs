use simple_useragent::UserAgent;

pub(crate) enum UserAgentType {
    PC,
    Pad,
    Mobile,
    Bot,
    Other,
}
pub(crate) fn classify_user_agent(ua: &UserAgent) -> UserAgentType {
    // 1. 先判断是否为爬虫/机器人
    if ua.client.family == "Spider"
        || ua.client.family.to_lowercase().contains("bot")
        || ua.client.family.to_lowercase().contains("spider")
        || ua.client.family == "Other" && {
            let lower = ua.client.family.to_lowercase();
            lower.contains("crawl") || lower.contains("slurp") || lower.contains("preview")
        }
    {
        return UserAgentType::Bot;
    }

    // 2. 根据 device.family 判断
    match ua.os.family.as_str() {
        // 明确的手机设备
        "iPhone" | "Windows Phone" | "BlackBerry" | "Generic Smartphone" => {
            return UserAgentType::Mobile;
        }

        // 明确的平板设备
        "iPad" | "Android Tablet" | "Kindle" | "Kindle Fire" | "Nexus 7" | "Generic Tablet" => {
            return UserAgentType::Pad;
        }

        // Android 情况较复杂：可能是手机也可能是平板
        "Android" => {
            // 简单启发：如果 UA 中包含 "mobile" 且不包含 "tablet" → mobile
            // 但 simple-useragent 已尽量区分，若 device.family 是 "Android" 而非 "Android Tablet"，通常视为 mobile
            return UserAgentType::Mobile;
        }

        // 桌面设备
        "Mac" | "Windows" | "Linux" | "Chrome OS" => {
            return UserAgentType::PC;
        }

        // 未知设备，尝试从 OS 判断
        "Other" => {
            let os = ua.os.family.to_lowercase();
            if os.contains("windows")
                || os.contains("mac")
                || os.contains("linux")
                || os.contains("chrome")
            {
                return UserAgentType::PC;
            } else if os.contains("ios") || os.contains("android") {
                // 可能是移动端，但未识别具体设备
                return UserAgentType::Mobile;
            }
        }

        _ => {}
    }

    UserAgentType::Other
}
