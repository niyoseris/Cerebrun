pub fn calculate_cost(provider: &str, model: &str, prompt_tokens: i32, completion_tokens: i32) -> f64 {
    let (input_per_1k, output_per_1k) = match (provider, model) {
        ("openai", m) if m.contains("gpt-4o-mini") => (0.00015, 0.0006),
        ("openai", m) if m.contains("gpt-4o") => (0.0025, 0.01),
        ("openai", m) if m.contains("gpt-4-turbo") => (0.01, 0.03),
        ("openai", m) if m.contains("gpt-4") => (0.03, 0.06),
        ("openai", m) if m.contains("gpt-3.5") => (0.0005, 0.0015),
        ("openai", m) if m.contains("o1-mini") => (0.003, 0.012),
        ("openai", m) if m.contains("o1") => (0.015, 0.06),
        ("gemini", m) if m.contains("1.5-flash") => (0.000075, 0.0003),
        ("gemini", m) if m.contains("1.5-pro") => (0.00125, 0.005),
        ("gemini", m) if m.contains("2.0-flash") => (0.0001, 0.0004),
        ("gemini", m) if m.contains("2.5-pro") => (0.00125, 0.01),
        ("anthropic", m) if m.contains("claude-3-5-sonnet") || m.contains("claude-3.5-sonnet") => (0.003, 0.015),
        ("anthropic", m) if m.contains("claude-3-5-haiku") || m.contains("claude-3.5-haiku") => (0.0008, 0.004),
        ("anthropic", m) if m.contains("claude-3-opus") => (0.015, 0.075),
        _ => (0.001, 0.002),
    };

    (prompt_tokens as f64 / 1000.0 * input_per_1k) + (completion_tokens as f64 / 1000.0 * output_per_1k)
}
