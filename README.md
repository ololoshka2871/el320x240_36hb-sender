# OBS virtual camera to GIP10000

# Setup
* Install [OBS](https://obsproject.com/)
* Setup output format 100x100 YUYV (YUV422)
* Setup virtual scene
* Start virtual web-camera
* Start this application
    Parameters:
    -p <port> - Virtual serial port with GIP10000 connected to
    -w <1-255> - "white" level [default 255]
    -b <0-254> - "black" level [default 0]
* Enjoy!