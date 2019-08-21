#[macro_use]
extern crate diesel;

mod api;
mod cache;
mod commands;
mod db;
mod dispatcher;
mod schema;
mod state_machine;
mod tags;

use cache::MessageCache;
use commands::{Args, Commands};
use dispatcher::{EventDispatcher, MessageDispatcher};
use serenity::{model::prelude::*, utils::parse_username, Client};
use std::str::FromStr;

type Result = crate::commands::Result<()>;

fn app() -> Result {
    let token = std::env::var("DISCORD_TOKEN")
        .map_err(|_| "missing environment variable: DISCORD_TOKEN")?;

    let _ = db::run_migrations()?;

    let mut cmds = Commands::new();

    // Tags
    cmds.add("?tag {key}", tags::get);
    cmds.add("?tag delete {key}", tags::delete);
    cmds.add("?tag create {key} [value]", tags::post);
    cmds.add("?tags", tags::get_all);

    // Slow mode.
    // 0 seconds disables slowmode
    cmds.add("?slowmode {channel} {seconds}", slow_mode);

    // Kick
    cmds.add("?kick {user}", kick);

    // Ban
    cmds.add("?ban {user}", ban);

    // Post the welcome message to the welcome channel.
    cmds.add("?CoC {channel}", welcome_message);

    cmds.add("?reset-welcome", reset_welcome_message);

    let messages = MessageDispatcher::new(cmds);

    let mut client =
        Client::new_with_handlers(&token, Some(messages), Some(EventDispatcher)).unwrap();

    client.start()?;

    Ok(())
}

fn main() {
    if let Err(err) = app() {
        eprintln!("error: {}", err);
        std::process::exit(1);
    }
}

/// Set slow mode for a channel.  
///
/// A `seconds` value of 0 will disable slowmode
fn slow_mode(args: Args) -> Result {
    if api::is_mod(&args)? {
        let seconds = &args
            .params
            .get("seconds")
            .ok_or("unable to retrieve seconds param")?
            .parse::<u64>()?;

        let channel_name = &args
            .params
            .get("channel")
            .ok_or("unable to retrieve channel param")?;

        ChannelId::from_str(channel_name)?.edit(&args.cx, |c| c.slow_mode_rate(*seconds))?;
    }
    Ok(())
}

/// Kick a user from the guild.  
///
/// Requires the kick members permission
fn kick(args: Args) -> Result {
    if api::is_mod(&args)? {
        let user_id = parse_username(
            &args
                .params
                .get("user")
                .ok_or("unable to retrieve user param")?,
        )
        .ok_or("unable to retrieve user id")?;

        if let Some(guild) = args.msg.guild(&args.cx) {
            guild.read().kick(&args.cx, UserId::from(user_id))?
        }
    }
    Ok(())
}

/// Ban an user from the guild.  
///
/// Requires the ban members permission
fn ban(args: Args) -> Result {
    if api::is_mod(&args)? {
        let user_id = parse_username(
            &args
                .params
                .get("user")
                .ok_or("unable to retrieve user param")?,
        )
        .ok_or("unable to retrieve user id")?;

        if let Some(guild) = args.msg.guild(&args.cx) {
            guild.read().ban(&args.cx, UserId::from(user_id), &"all")?
        }
    }
    Ok(())
}

/// Write the welcome message to the welcome channel.  
fn welcome_message(args: Args) -> Result {
    const WELCOME_BILLBOARD: &'static str = "By participating in this community, you agree to follow the Rust Code of Conduct, as linked below. Please click the :white_check_mark: below to acknowledge, or otherwise you may leave this Discord.

  https://www.rust-lang.org/policies/code-of-conduct  ";

    if api::is_mod(&args)? {
        let channel_name = &args
            .params
            .get("channel")
            .ok_or("unable to retrieve channel param")?;

        let channel_id = ChannelId::from_str(channel_name)?;
        let message = channel_id.say(&args.cx, WELCOME_BILLBOARD)?;
        let white_check_mark = ReactionType::from("✅");
        message.react(&args.cx, white_check_mark)?;
        MessageCache::save("welcome", message.id, channel_id)?;
    }
    Ok(())
}

fn reset_welcome_message(args: Args) -> Result {
    if api::is_mod(&args)? {
        MessageCache::delete_by_name("welcome")?;
    }
    api::send_reply(
        &args,
        "welcome message reset, you can now create a new welcome message with the ?CoC command",
    )?;
    Ok(())
}
