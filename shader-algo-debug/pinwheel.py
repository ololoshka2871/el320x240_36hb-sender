#!/usr/bin/env python3


import struct
import turtle
import numpy as np


def setwindowsize(x=640, y=640):
    turtle.setup(x, y)
    turtle.setworldcoordinates(0,0,x,y)


def drawpixel(x, y, color, pixelsize = 1 ):
    turtle.tracer(0, 0)
    turtle.colormode(255)
    turtle.penup()
    turtle.setpos(x*pixelsize,y*pixelsize)
    turtle.color(color)
    turtle.pendown()
    turtle.begin_fill()
    for i in range(4):
        turtle.forward(pixelsize)
        turtle.right(90)
    turtle.end_fill()


def draw_gradient(r, heigth, gen_pixel):
    turtle.tracer(0, 0)
    turtle.colormode(255)
    turtle.penup()
    turtle.seth(270)
    for line in r:
        color = gen_pixel(line)
        turtle.setpos(line, 100)
        turtle.color(color)
        turtle.pendown()
        turtle.begin_fill()
        turtle.fd(heigth)
        turtle.end_fill()
        turtle.penup()


def showimage():
    turtle.hideturtle()
    turtle.update()


def hex2rgb(hexcode):
    v = int(hexcode[1:], 16)
    p = struct.pack(">I", v)
    return p[1:]


def get_r(color):
    return color[0]


def get_pixel_color(x, y):
    # canvas use different coordinates than turtle
    y = -y

    # get access to tkinter.Canvas
    canvas = turtle.getcanvas()

    # find IDs of all objects in rectangle (x, y, x, y)
    ids = canvas.find_overlapping(x, y, x, y)

    # if found objects
    if ids:
        # get ID of last object (top most)
        index = ids[-1]

        # get its color
        color = canvas.itemcget(index, "fill")

        # if it has color then return it
        if color:
            return get_r(hex2rgb(color))

    # if there was no object then return "white" - background color in turtle
    return 0xff  # default color

#-----------------------------------------------------------------------------------------------------------------------

# Шейдер

def shader(config, out_tex, global_id):
    step_types = [
        0,
        1,
        2, 2,
        3, 3,
        0, 0, 0,
        1, 1, 1,
        2, 2, 2, 2,
    ]
    a = 7.0 / 16.0
    b = 3.0 / 16.0
    c = 5.0 / 16.0
    d = 1.0 / 16.0

    matrixes = [
        # down
        np.array([  [0.0, 0.0, b],
                    [0.0, 0.0, c],
                    [0.0, a, d]]
                 ),
        # left
        np.array([  [0.0, 0.0, 0.0],
                    [a, 0.0, 0.0],
                    [d, c, b]]
                 ),
        # up
        np.array([  [d, a, 0.0],
                    [c, 0.0, 0.0],
                    [b, 0.0, 0.0]]
                 ),
        # right
        np.array([  [d, c, b],
                    [0.0, 0.0, a],
                    [0.0, 0.0, 0.0]]
                 )
    ]

    blocks_per_x = config["width"] // (4 + 4)

    block_num = global_id

    # Начальная точка спирали
    x = 3 + ((4 + 4) * block_num) % blocks_per_x
    y = 1 + block_num // blocks_per_x

    output_tex_dim = np.array([100, 255])

    start_point = np.array([x - 2, y - 1])
    for x_add in range(4):
        for y_add in range(4):
            point_coords = start_point + np.array([x_add, y_add])

            tex_coords = point_coords / output_tex_dim
            gray = get_pixel_color(tex_coords[0], tex_coords[1])
            print(f"color {gray}")

            out_tex[point_coords[0], point_coords[1]] = gray


#-----------------------------------------------------------------------------------------------------------------------


def main():
    block_size = (8, 4)
    config = {"width": 0xff, "height": 100}
    setwindowsize(255, 50)
    draw_gradient(range(config["width"]), config["height"], lambda x: (x, x, x))
    showimage()

    outbuf_texture = np.empty((config["width"], config["height"]), dtype=np.uint8)

    for ti in range(2):
        shader(config, outbuf_texture, ti)

    turtle.getscreen()._root.mainloop()  # Don't close window


if __name__ == "__main__":
    main()
