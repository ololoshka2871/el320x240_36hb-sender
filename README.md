# OBS virtual camera to el320x240_36hb

# Setup
* Install [OBS](https://obsproject.com/)
* Setup output format 320x240 YUYV (YUV422)
* Setup virtual scene
* Start virtual web-camera
* Start this application
    Parameters:
    -p <port> - Virtual serial port with el320x240_36hb connected to
    -w <1-255> - "white" level [default 255]
    -b <0-254> - "black" level [default 0]
* Enjoy!

# Algorithm
1. Считываем из камеры кадр любого размрера `self.camera.frame()`. и декодируем его как `LumaFormat`: `frame.decode_image::<pixel_format::LumaFormat>()`.  Возвращается одномерный массив `u8` размером `width * height`, с яркостью каждого пикселя (u8).
2. Записываем полученый массив в текстуру в памяти видеокарты: `queue.write_texture()`. 
    * Текстура создана со следующими свойствами:
        - Размер - размер кадра с камеры
        - Формат - `R8Unorm` - один байт на пиксель, внутри шейдера он будет в диопазоне от 0.0 до 1.0
        - Мипмапы - нет
        - Семплирование - `Nearest` - ближайший сосед
        - Обертка - `ClampToEdge` - не повторять текстуру
    * Создем пустую текстуру, в которую будем записывать результаты вычислений.
        - Размер - 320x240
        - Формат - `R8Unorm` - один байт на пиксель, внутри шейдера он будет в диопазоне от 0.0 до 1.0
        - Мипмапы - нет
        - Семплирование - `Nearest` - ближайший сосед
        - Обертка - `ClampToEdge` - не повторять текстуру
    * Создем пустой буфер, в который будем записывать результаты для передачи в `el320x240_36hb`.
        - Размер - 320 * 240 / 8 = 9600 байт
3. Запускаем вычислительный шейдер, который преодразует входную текстуреу в 2 объекта:
    1. Текстуру фиксированного размера 320x240, с форматом `R8Unorm`.
    2. Стореджбуфер, в котором будут храниться данные для передачи в `el320x240_36hb`. Размер стореджбуфера - 320 * 240 / 8 = 9600 байт.
4. Вычислительный шейдер:
    1. Обходит все точки выходной текстуры
    2. Считывает яркость пикселя из входной текстуры, и передает её на вычисление дизеринга, получается яркость пикселя 0.0 или 1.0.
    3. Записывается получено значение в выходную текстуру.
    4. Если яркость пикселя 1.0, то в стореджбуфер в нухный бит записывается 1.
5. Читаем стореджбуфер и отправляем его в `el320x240_36hb` через виртуальный COM-порт.
6. Фрагментный шейдер графического пайплайна читает выходную текстуру и отображает её на экране в окне предпросмотра.

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
6. Add env to build: `$env:LIBCLANG_PATH="C:/path/to/clang/bin"`
7. Buld the project `cargo build --release`