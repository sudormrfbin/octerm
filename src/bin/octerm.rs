use octerm::{error::Error, network::start_server, OctermApp, ServerRequest, ServerResponse};

pub fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    let token = std::env::var("GITHUB_TOKEN").map_err(|_| Error::Authentication)?;

    // Initialise a statically counted instance
    let builder = octocrab::Octocrab::builder().personal_token(token);
    octocrab::initialise(builder)?;

    let (server_channel, app_channel) = meow::server::channels::<ServerRequest, ServerResponse>();
    std::thread::spawn(move || {
        start_server(server_channel);
    });

    app_channel.send_to_server(ServerRequest::RefreshNotifs)?;
    meow::run::<OctermApp>(Some(app_channel))?;
    Ok(())
}
