x:
    cargo run --example weather_client

reset-db:
    cargo sqlx database reset --force -y