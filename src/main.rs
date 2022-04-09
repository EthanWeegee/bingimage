use std::{
	fs::File,
	path::PathBuf,
	sync::Arc, io::Write
};
use clap::{ Error, Arg, Command };
use serde_json::Value as JsonValue;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	let mut app = Command::new("bingimage")
		.about("Downloads the Bing image of the day")
		.arg(Arg::new("resolution")
			.short('r')
			.help("Image resolution")
			.long_help("Image resolution, formatted as WIDTHxHEIGHT. ex. 1920x1080\nThis argument can be passed multiple times for as many resolutions as you need")
			.required(true)
			.multiple_occurrences(true)
			.takes_value(true)
			.multiple_values(false)
		)
		.arg(Arg::new("readme")
			.short('m')
			.help("Output README.md")
			.long_help("Output README.md with title and copyright information")
			.takes_value(false)
		)
		.arg(Arg::new("path")
			.short('p')
			.help("Output directory")
			.long_help("Directory of the output files")
			.takes_value(true)
			.required(true)
			.multiple_values(false)
			.multiple_occurrences(false)
		);
	let res_error = app.error(clap::ErrorKind::InvalidValue, "Can't parse resolution value. See help for information on how to format it.");
	let path_error = app.error(clap::ErrorKind::InvalidValue, "Output path must be a directory.");
	let app = app.get_matches();

	let mut resolutions: Vec<Arc<Resolution>> = Vec::new();

	match app.values_of("resolution") {
		Some(values) => {
			for value in values {
				let split: Vec<&str> = value.split('x').collect();
				if split.len() != 2 {
					Error::exit(&res_error)
				}

				match (split[0].parse::<u16>(), split[1].parse::<u16>()) {
					(Err(_), _) => Error::exit(&res_error),
					(_, Err(_)) => Error::exit(&res_error),
					(x, y) => {
						resolutions.push(Arc::new(Resolution::new(x.unwrap(), y.unwrap())))
					}
				}
			}
		},
		// Unreachable? This argument is marked as required
		None => Error::exit(&res_error)
	}

	// Get the path from the arguments
	let path = Arc::new(app.value_of_t_or_exit::<PathBuf>("path"));
	// Enforce that the path is a directory
	if !path.is_dir() {
		Error::exit(&path_error)
	}

	let json = reqwest::get("https://www.bing.com/HPImageArchive.aspx?format=js&idx=0&n=1")
		.await?
		.json::<JsonValue>()
		.await?;

	let meta = &json["images"][0];

	let url = Arc::new(meta["url"].to_string().trim_matches('"').to_string());
	let title = Arc::new(meta["title"].to_string().trim_matches('"').to_string());
	let copyright = Arc::new(meta["copyright"].to_string().trim_matches('"').to_string());

	let mut handles = Vec::new();

	for resolution in resolutions {
		let properties = ImageProperties {
			resolution: resolution,
			url: url.clone(),
			title: title.clone(),
			copyright: copyright.clone()
		};
		handles.push(tokio::spawn(download(properties, path.clone())))
	}
	if app.is_present("readme") {
		let properties = ImageProperties {
			resolution: Arc::new(Resolution::new(0, 0)),
			url: url.clone(),
			title: title.clone(),
			copyright: copyright.clone()
		};
		handles.push(tokio::spawn(create_metadata(properties, path.clone())))
	}

	for handle in handles {
		tokio::try_join!(handle)?;
	}
	Ok(())
}

/// Download an image with [ImageProperties] to a specified path.
/// 
/// Doesn't return an error, but will print any errors it gets to stderr.
async fn download(properties: ImageProperties, path: Arc<PathBuf>) {
	let res_string = properties.resolution.to_string();
	let file_name = format!("{}.jpg", res_string);
	let file_path = path.join(&file_name);
	
	// Replace the resolution in the image path with our own
	let url = properties.url.replace("1920x1080", &res_string);
	// Add the base URL
	let url = format!("https://bing.com{}", url);
	let image = reqwest::get(&url).await;

	match image {
		Ok(response) => {
			match response.bytes().await {
				Ok(bytes) => {
					match File::create(&file_path) {
						Ok(mut file) => {
							match file.write(&bytes) {
								Ok(len) => {
									if len > bytes.len() {
										eprintln!("Error writing file {:?}: entire file may not have been written", file_path);
										return
									}
								},
								Err(error) => {
									eprintln!("Error writing file {:?}: {}", file_path, error);
									return
								}
							};

							match file.sync_all() {
								Ok(_) => {
									println!("Successfully written file {:?}", file_path);
									return
								},
								Err(error) => {
									eprintln!("Error writing file {:?}: {}", file_path, error);
									return
								}
							};
						},
						Err(error) => {
							eprintln!("Error creating file {:?}: {}", file_path, error);
							return
						}
					}
				},
				Err(error) => {
					eprintln!("Error downloading from \"{}\": {}", url, error);
					return
				}
			}
		},
		Err(error) => {
			eprintln!("Error downloading from \"{}\": {}", url, error);
			return
		}
	}
}

/// Write a markdown file with properties from the [ImageProperties] to a specified path.
/// 
/// Doesn't make any http connections. Doesn't return an error, but will print any errors it gets to stderr. 
async fn create_metadata(properties: ImageProperties, path: Arc<PathBuf>) {
	let metadata_md = format!("# {}\n## {}\n", properties.title, properties.copyright);
	let file_name = "README.md";
	let file_path = path.join(file_name);

	match File::create(&file_path) {
		Ok(mut file) => {
			match file.write(metadata_md.as_bytes()) {
				Ok(len) => {
					if len > metadata_md.as_bytes().len() {
						eprintln!("Error writing file {:?}: entire file may not have been written", file_path);
						return
					}
				},
				Err(error) => {
					eprintln!("Error writing file {:?}: {}", file_path, error);
					return
				}
			};

			match file.sync_all() {
				Ok(_) => {
					println!("Successfully written file {:?}", file_path);
					return
				},
				Err(error) => {
					eprintln!("Error writing file {:?}: {}", file_path, error);
					return
				}
			};
		},
		Err(error) => {
			eprintln!("Error creating file {:?}: {}", file_path, error);
			return
		}
	}
}

struct Resolution {
	pub x: u16,
	pub y: u16
}

impl Resolution {
	pub fn new(x: u16, y: u16) -> Self {
		Self { x, y }
	}

	pub fn to_string(&self) -> String {
		format!("{}x{}", self.x, self.y)
	}
}

struct ImageProperties {
	pub resolution: Arc<Resolution>,
	pub url: Arc<String>,
	pub title: Arc<String>,
	pub copyright: Arc<String>
}