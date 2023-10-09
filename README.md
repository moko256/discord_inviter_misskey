## discord_inviter_misskey
Simple Misskey and Discord bot, which send Discord guild invitation URL to Misskey user who is in the same instance.

### Features
Here, bot username at Misskey is `@bot`.
- Reply invitation URL to the mention `@bot ...`.

### Usage
- Production
```bash
installation_target="you/favorite/dir"
mkdir -p $installation_target
cargo build --release
cp target/release/discord_inviter_misskey* $installation_target/
cp bot_config-template.toml $installation_target/bot_config.toml
cd $installation_target
# Edit `bot_config.toml` here.
# Run `discord_inviter_misskey[.exe] here`.
```

- Debug
```bash
cp bot_config-template.toml bot_config.toml
# Edit `bot_config.toml` here.
cargo run
```

### License
SPDX-License-Identifier: AGPL-3.0-or-later
