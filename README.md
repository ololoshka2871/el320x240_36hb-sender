# OBS virtual camera to el320x240_36hb

# Setup
* Install [OBS](https://obsproject.com/)
* Setup stream target rtmp://127.0.0.1:9009
* Setup virtual scene
* Start this application
    Parameters:
    - `-r <port>` - RTMP listen port
    - `-s <port>` - Virtual serial port with el320x240_36hb connected to 
    - `-f <algo>` - Select [select dithering algorithm](https://ffmpeg.org/ffmpeg-filters.html#toc-paletteuse)
    - `-t <fmt>` - Select pixel format `monob` (default) or `monow`
* Start OBS Stream.
* Enjoy!

## Build
1. Install [vcpkg](https://github.com/microsoft/vcpkg):
    ```ps
    git clone https://github.com/microsoft/vcpkg.git
    cd vcpkg
    .\bootstrap-vcpkg.bat
    ```
2. Add `vcpkg` to system `PATH`
3. In this project dirrectory: `vcpkg integrate install`
4. Build and Install `ffmpeg`: `vcpkg install ffmpeg:x64-windows-static-md`
5. Install [clang](https://github.com/llvm/llvm-project/releases)
6. Add `env` to build: `$env:LIBCLANG_PATH="C:/path/to/clang/bin"`
7. Buld the project `cargo build --release`