# NoPixel Live

See every live NoPixel streamer on Twitch and Kick in one window. Click a stream to watch it in VLC.

**[Download for Windows](../../releases/latest)**

- One list of everyone playing NoPixel right now, on both sites.
- A streamer who is live on Twitch and Kick at the same time shows once, with a button for each site.
- Star your favorites. They stay at the top, and Windows tells you when they go live.
- Search by streamer name or character name.
- Hide streamers you do not want to see.
- No account and no login.

## Install

You need Windows 10 or 11. Setup takes about 5 minutes.

1. Install **Streamlink**. It gets the video from Twitch and Kick. Open the [Streamlink download page](https://github.com/streamlink/windows-builds/releases/latest), click the file that ends in `x86_64.exe`, and run it.
2. Install **VLC**. It plays the video. Get it from [videolan.org](https://www.videolan.org/vlc/).
3. Open the [NoPixel Live download page](../../releases/latest). Under **Assets**, click the file that ends in `setup.exe`.
4. Run the file you downloaded.

Windows may say "Windows protected your PC". It says this because the app is not signed with a paid certificate. Click **More info**, then **Run anyway**.

## Watch a stream

- Click the picture of a stream to play it. VLC opens after a few seconds.
- Click **Twitch** or **Kick** under a stream to choose the site.
- Click the small box with an arrow next to a site to open the stream in your web browser.
- Click the star to make a streamer a favorite.
- Point at a stream and click the eye to hide that streamer. Click **Undo** if you change your mind.
- Type in the search box to find a streamer or a character. Press `/` to jump to the search box.
- Click **All**, **Twitch**, **Kick**, or **Both** to filter the list. **Both** shows streamers who are live on both sites.

The list updates every minute. Click the round arrow at the top right to update it now.

## The tray icon

When you close the window, the app keeps running in the tray next to the clock. This keeps favorite notifications coming. Point at the icon to see how many streamers are live. To close the app fully, right-click the icon and click **Quit**.

To quit when you close the window instead, open Settings and turn off **Keep running in the tray**.

## Settings

Click the sliders icon at the top right. Click **Save** when you are done.

| Setting | What it does |
| --- | --- |
| Streamlink and Player | Where Streamlink and your video player are installed. Leave them empty and the app finds them. mpv also works as a player. |
| Quality | Video quality. `best` is the highest. Pick `720p60,best` to use less internet data. |
| Play first on | Which site the picture plays when a streamer is live on both. |
| Extra Streamlink options | Extra options for Streamlink, for example `--twitch-low-latency`. |
| Twitch tags | The Twitch tags that mark a NoPixel stream. |
| Kick event | The Kick event page that lists NoPixel streams. |
| Only this category | Hides streams of other games. The default is Grand Theft Auto V. Leave it empty to show every game. |
| Check every | How often the list updates, from 30 to 600 seconds. |
| Added channels | Streamers to show whenever they play the game, even if their title does not say NoPixel. Paste names or links. |
| Hidden | Streamers you hid. Click the **×** to show them again. |
| Favorites | Add a favorite by name, even when they are offline. |
| Linked names | Joins one streamer who uses a different name on Twitch and on Kick. |

## Fix a problem

**"Streamlink was not found"**
Install Streamlink (step 1 above). If you installed it in a different folder, open Settings and enter the full path to `streamlink.exe`.

**A stream does not play**
The app shows the reason at the bottom left. Check that VLC is installed. For Kick, Streamlink needs Microsoft Edge or Google Chrome. Edge comes with Windows, so this works unless you removed it.

**A streamer is missing**
The app finds Twitch streams by their NoPixel tags and Kick streams by the NoPixel event page. If a streamer uses neither, open Settings and add them under **Added channels**.

**A streamer shows twice**
They use a different name on each site. Open Settings and join the two names under **Linked names**.

**A message under the top bar says Twitch or Kick failed**
The app reads the same data that the Twitch and Kick websites use, and those sites sometimes change or block requests for a while. The other site keeps working. Wait a few minutes, then click the round arrow. If the error stays for days, the app needs an update.

## Update or remove

To update, download the newest setup file from the [download page](../../releases/latest) and run it. Your settings stay.

To remove the app, open Windows **Settings**, go to **Apps** > **Installed apps**, find NoPixel Live, and click **Uninstall**.

## Privacy

- The app has no account and never asks for a password.
- It only talks to Twitch and Kick, the same way their websites do.
- Your settings stay on your computer, in `%APPDATA%\dev.nopixellive.app\settings.json`.

## About

NoPixel Live is a fan project. It is not made by or connected to NoPixel, Twitch, or Kick.

To build the app yourself or change the code, see [DEVELOPMENT.md](DEVELOPMENT.md).
