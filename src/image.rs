
pub async fn pull(image: &str, tag: &str) -> anyhow::Result<String> {
    let token = get_token(image).await?;
    let manifest = get_manifest(image, tag, &token).await?;
    let layers = get_image_manifest(image, &manifest, &token).await?;
    for layer in &layers {
        let filename = layer.trim_matches('"').replace("sha256:","");
        let path = format!("/tmp/runic/layers/{}.tar.gz",filename);
        if std::path::Path::new(&path).exists(){
            println!("already have the image");
            continue;
        }
        download_layer(image, &layer, &token).await?;
    }
    let rootfs = extract_layer(image, tag, &layers)?;
    Ok(rootfs)
}

async fn get_token(image: &str) -> anyhow::Result<String> {
    let url = format!(
        "https://auth.docker.io/token?service=registry.docker.io&scope=repository:{image}:pull",
        image = image
    );

    let response = reqwest::get(&url)
        .await?
        .json::<serde_json::Value>()
        .await?;
    println!("Got token successfully");
    Ok(response["token"].as_str().ok_or_else(|| anyhow::anyhow!("Token not found in response"))?.to_string())
}

async fn get_manifest(image: &str, tag: &str, token: &str)-> anyhow::Result<String> {
    let url = format!(
        "https://registry-1.docker.io/v2/{image}/manifests/{tag}",
        image = image,
        tag = tag
    );

    let client = reqwest::Client::new();
    let response = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", token))
        .header("Accept", "application/vnd.docker.distribution.manifest.v2+json")
        .send()
        .await?
        .json::<serde_json::Value>()
        .await?;

    let layers = response["manifests"].as_array().ok_or_else(|| anyhow::anyhow!("Layers not found in manifest"))?;
    let my_arch = match std::env::consts::ARCH {
        "x86_64" => "amd64",
        "aarch64" => "arm64",
        other => other,
    };

    let my_os = std::env::consts::OS;
    for layer in layers {
        if layer["platform"]["architecture"].as_str() == Some(my_arch) && layer["platform"]["os"].as_str() == Some(my_os) {
            println!("Found matching layer for architecture {} and OS {}", my_arch, my_os);
            return Ok(layer["digest"].as_str().ok_or_else(|| anyhow::anyhow!("Layer digest not found"))?.to_string());
        }
    }
    Err(anyhow::anyhow!("No matching layer found"))
}

async fn get_image_manifest(image: &str, digest: &str, token: &str) -> anyhow::Result<Vec<String>>{
    let url = format!(
        "https://registry-1.docker.io/v2/{image}/manifests/{digest}",
        image = image,
        digest = digest
    );

    let client = reqwest::Client::new();
    let response = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await?
        .json::<serde_json::Value>()
        .await?;

    let layers = response["layers"].as_array().ok_or_else(|| anyhow::anyhow!("Layers not found in image manifest"))?;
    let digests = layers.iter().map(|l| l["digest"].as_str().unwrap_or("").to_string()).collect();
    Ok(digests)
}

async fn download_layer(image: &str, digest: &str, token: &str) -> anyhow::Result<()>{
    let url = format!(
        "https://registry-1.docker.io/v2/{image}/blobs/{digest}",
        image = image,
        digest = digest
    );

    let client = reqwest::Client::new();
    let response = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await?
        .bytes()
        .await?;
    let filename = digest.trim_matches('"').replace("sha256:", "");
    let path = format!("/tmp/runic/layers/{}.tar.gz", filename);

    std::fs::create_dir_all("/tmp/runic/layers")?;
    std::fs::write(&path, &response)?;
    println!("Downloaded layer {} to {}", digest, path);
    Ok(())
}

fn extract_layer(image: &str, tag: &str, layers_digests: &[String]) -> anyhow::Result<String> {
    let rootfs_path = format!("/tmp/runic/rootfs/{}/{}", image.replace("/", "_"), tag);
    std::fs::create_dir_all(&rootfs_path)?;

    for digest in layers_digests{
        let filename = digest.trim_matches('"').replace("sha256:","");
        let filepath = format!("/tmp/runic/layers/{}.tar.gz",filename);

        let output = std::process::Command::new("sudo")
            .args(["tar", "xzf", &filepath, "-C", &rootfs_path])
            .output()?;
        if !output.status.success(){
            let err = String::from_utf8_lossy(&output.stderr);
            println!("tar warning: {}", err);
        }
    }
    Ok(rootfs_path)
}