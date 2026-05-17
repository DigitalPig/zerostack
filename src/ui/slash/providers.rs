use crate::config;
use crate::ui::slash::{SlashCtx, write_error, write_ok, write_result};

pub async fn handle(parts: &[&str], ctx: &mut SlashCtx<'_>) -> anyhow::Result<()> {
    match parts[0] {
        "/provider" => handle_provider(parts, ctx).await,
        "/model" => handle_model(parts, ctx).await,
        "/models" => handle_models(parts, ctx).await,
        "/models-add" => handle_models_add(parts, ctx).await,
        _ => Ok(()),
    }
}

async fn handle_provider(parts: &[&str], ctx: &mut SlashCtx<'_>) -> anyhow::Result<()> {
    if parts.len() < 2 {
        write_ok(
            ctx.renderer,
            format!("current provider: {}", ctx.session.provider),
        );
        return Ok(());
    }
    let new_provider = parts[1].trim();
    if crate::provider::parse_provider(new_provider).is_none()
        && !ctx.cfg.custom_providers_map().contains_key(new_provider)
    {
        write_error(
            ctx.renderer,
            format!("unknown provider: '{}'", new_provider),
        );
        return Ok(());
    }
    ctx.rebuild_agent_with_client(new_provider, *ctx.reasoning_enabled)
        .await?;
    ctx.session.provider = compact_str::CompactString::new(new_provider);
    write_ok(
        ctx.renderer,
        format!("switched to provider: {}", new_provider),
    );
    Ok(())
}

async fn handle_model(parts: &[&str], ctx: &mut SlashCtx<'_>) -> anyhow::Result<()> {
    if parts.len() < 2 {
        write_ok(
            ctx.renderer,
            format!("current model: {}", ctx.session.model),
        );
        let custom = ctx.cfg.custom_providers_map();
        let models = crate::provider::fetch_available_models(
            ctx.cli.resolve_provider(ctx.cfg).as_str(),
            &custom,
        )
        .await;
        if models.is_empty() {
            write_ok(
                ctx.renderer,
                "(no model list for this provider — use /model <name>)",
            );
        } else {
            ctx.input.start_model_picker(models, ctx.session.model.as_str());
        }
        return Ok(());
    }
    let new_model = compact_str::CompactString::new(parts[1].trim());
    let model = ctx.client.completion_model(new_model.to_string());
    *ctx.agent = Some(
        crate::provider::build_agent(
            model,
            ctx.cli,
            ctx.cfg,
            ctx.context,
            ctx.permission.clone(),
            ctx.ask_tx.clone(),
            ctx.sandbox.clone(),
            *ctx.reasoning_enabled,
            #[cfg(feature = "mcp")]
            ctx.mcp_manager,
        )
        .await,
    );
    ctx.session.model = new_model.clone();
    ctx.session.provider = ctx.cli.resolve_provider(ctx.cfg);
    write_ok(ctx.renderer, format!("switched to model: {}", new_model));
    Ok(())
}

async fn handle_models(parts: &[&str], ctx: &mut SlashCtx<'_>) -> anyhow::Result<()> {
    let qm = config::quick_models_map(ctx.cfg);
    let mut sorted: Vec<&String> = qm.keys().collect();
    sorted.sort();
    if parts.len() < 2 {
        if sorted.is_empty() {
            write_ok(ctx.renderer, "no quick models defined");
        } else {
            write_ok(
                ctx.renderer,
                format!(
                    "quick models (current: {} | {}):",
                    ctx.session.provider, ctx.session.model
                ),
            );
            for name in &sorted {
                let q = &qm[name.as_str()];
                write_result(
                    ctx.renderer,
                    format!("  {}  ({} / {})", name, q.provider, q.model),
                );
            }
        }
    } else {
        let name = parts[1].trim();
        if let Some(q) = qm.get(name) {
            ctx.rebuild_agent_with_client(&q.provider, *ctx.reasoning_enabled)
                .await?;
            let model = ctx.client.completion_model(q.model.to_string());
            *ctx.agent = Some(
                crate::provider::build_agent(
                    model,
                    ctx.cli,
                    ctx.cfg,
                    ctx.context,
                    ctx.permission.clone(),
                    ctx.ask_tx.clone(),
                    ctx.sandbox.clone(),
                    *ctx.reasoning_enabled,
                    #[cfg(feature = "mcp")]
                    ctx.mcp_manager,
                )
                .await,
            );
            ctx.session.provider = compact_str::CompactString::new(&q.provider);
            ctx.session.model = compact_str::CompactString::new(&q.model);
            write_ok(
                ctx.renderer,
                format!(
                    "switched to quick model: {} ({} / {})",
                    name, q.provider, q.model
                ),
            );
        } else {
            write_error(ctx.renderer, format!("unknown quick model: '{}'", name));
            if !sorted.is_empty() {
                write_ok(ctx.renderer, "available quick models:");
                for n in &sorted {
                    write_result(ctx.renderer, format!("  {}", n));
                }
            }
        }
    }
    Ok(())
}

async fn handle_models_add(parts: &[&str], ctx: &mut SlashCtx<'_>) -> anyhow::Result<()> {
    if parts.len() < 3 {
        write_ok(ctx.renderer, "usage: /models-add <name> <provider> <model>");
        return Ok(());
    }
    let name = parts[1].trim().to_string();
    let rest = parts[2].trim();
    let (provider, model) = match rest.split_once(' ') {
        Some((p, m)) => (p.trim().to_string(), m.trim().to_string()),
        None => {
            write_ok(ctx.renderer, "usage: /models-add <name> <provider> <model>");
            return Ok(());
        }
    };
    if name.is_empty() || provider.is_empty() || model.is_empty() {
        write_ok(ctx.renderer, "usage: /models-add <name> <provider> <model>");
        return Ok(());
    }
    match config::save_quick_model(&name, &provider, &model) {
        Ok(()) => {
            write_ok(
                ctx.renderer,
                format!("saved quick model: {} ({} / {})", name, provider, model),
            );
        }
        Err(e) => {
            write_error(ctx.renderer, format!("failed to save quick model: {}", e));
        }
    }
    Ok(())
}
