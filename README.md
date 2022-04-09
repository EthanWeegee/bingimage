# bingimage
A simple program that downloads the Bing image of the day.

```
USAGE:
    bingimage [OPTIONS] -r <resolution> -p <path>

OPTIONS:
    -h, --help
            Print help information

    -m
            Output README.md with title and copyright information

    -p <path>
            Directory of the output files

    -r <resolution>
            Image resolution, formatted as WIDTHxHEIGHT. ex. 1920x1080
            This argument can be passed multiple times for as many resolutions as you need
```

I use this to download the image of the day for use as my desktop wallpaper. This is a rewrite of a bash script I had for the same purpose. If you're curious, it looked like this:

```bash
#!/usr/bin/env bash
path=$BING_IMAGE_PATH
url=$(curl -s http://www.bing.com/HPImageArchive.aspx\?format\=js\&idx\=0\&n\=1 | jq -r '.images[0].url')
title=$(curl -s http://www.bing.com/HPImageArchive.aspx\?format\=js\&idx\=0\&n\=1 | jq -r '.images[0].title')
copyright=$(curl -s http://www.bing.com/HPImageArchive.aspx\?format\=js\&idx\=0\&n\=1 | jq -r '.images[0].copyright')

for res in 1920x1080 1366x768; do
	curl -s http://bing.com${url//1920x1080/${res}} > /tmp/${res}.jpg

	convert -comment "${title} \| ${copyright}" /tmp/${res}.jpg ${path}${res}.jpg
	rm /tmp/${res}.jpg
done

echo -e "# ${title}\n## ${copyright}" > ${path}README.md
```

This mostly just exists as a fun project. I wanted to learn some basics of how concurrency worked in real-world examples. While it's definitely overkill here, it was at least a little bit of a learning experience.

## systemd
If you use systemd, you can create a user service that looks something like this:
```
# bingimage.service
[Unit]
Description=Downloads the Bing Daily Wallpaper
After=network.target
Wants=bingimage.timer

[Service]
User=wwwrun
Group=www
Type=oneshot
ExecStart=/path/to/bingimage -p /path/to/bing/images -r 1920x1080

[Install]
WantedBy=timers.target
```
and then a timer like this:
```
# bingimage.timer
[Unit]
Description=Downloads the Bing Daily Wallpaper daily
After=network.target

[Timer]
Unit=bingimage.service
OnCalendar=*-*-* 4:00:00
Persistent=true

[Install]
WantedBy=timers.target
```
to download the wallpaper daily at 4 AM.