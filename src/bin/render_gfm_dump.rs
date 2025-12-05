use std::{env, error::Error, fs, path::Path};

use soffio::application::render::{RenderRequest, RenderService, RenderTarget, render_service};

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = env::args().skip(1);

    let mut sanitize = false;
    let first = args
        .next()
        .expect("usage: render_gfm_dump [--sanitize|--no-sanitize] <post|page> <markdown_path>");

    let (target_arg, path_arg) = match first.as_str() {
        "--sanitize" => {
            sanitize = true;
            let target = args.next().expect(
                "usage: render_gfm_dump [--sanitize|--no-sanitize] <post|page> <markdown_path>",
            );
            let path = args.next().expect(
                "usage: render_gfm_dump [--sanitize|--no-sanitize] <post|page> <markdown_path>",
            );
            (target, path)
        }
        "--no-sanitize" => {
            sanitize = false;
            let target = args.next().expect(
                "usage: render_gfm_dump [--sanitize|--no-sanitize] <post|page> <markdown_path>",
            );
            let path = args.next().expect(
                "usage: render_gfm_dump [--sanitize|--no-sanitize] <post|page> <markdown_path>",
            );
            (target, path)
        }
        other => {
            let target = other.to_string();
            let path = args.next().expect(
                "usage: render_gfm_dump [--sanitize|--no-sanitize] <post|page> <markdown_path>",
            );
            (target, path)
        }
    };

    if args.next().is_some() {
        panic!("usage: render_gfm_dump [--sanitize|--no-sanitize] <post|page> <markdown_path>");
    }

    let markdown = fs::read_to_string(&path_arg)?;
    let slug = Path::new(&path_arg)
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("gfm-fixture")
        .to_owned();

    let render_target = match target_arg.as_str() {
        "post" => RenderTarget::PostBody { slug },
        "page" => RenderTarget::PageBody { slug },
        other => {
            eprintln!("unknown target '{other}', expected 'post' or 'page'");
            std::process::exit(2);
        }
    };

    let public_site_url =
        env::var("PUBLIC_SITE_URL").unwrap_or_else(|_| "http://localhost:3000/".to_string());

    let renderer = render_service();
    let request = RenderRequest::new(render_target, markdown).with_public_site_url(public_site_url);

    if sanitize {
        let output = renderer.render(&request)?;
        println!("{}", output.html);
    } else {
        let html = renderer.render_unsanitized(&request)?;
        println!("{html}");
    }
    Ok(())
}
