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
2. Записываем полученый массив в текстуру в памяти видеокарты: `queue.write_texture()`. Текстура создана со следующими свойствами:
    - Размер - размер кадра с камеры
    - Формат - `R8Unorm` - один байт на пиксель, внутри шейдера он будет в диопазоне от 0.0 до 1.0
    - Мипмапы - нет
    - Семплирование - `Nearest` - ближайший сосед
    - Обертка - `ClampToEdge` - не повторять текстуру
3. Запускаем вычислительный шейдер, который преодразует входную текстуреу в текстуру фиксированного размера 320x240, с форматом `R8Unorm`.
    1. Обходит все точки выходной текстуры


