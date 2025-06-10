#!/usr/bin/python3

import time
import subprocess

import numpy as np

from picamera2 import Picamera2
from picamera2.encoders import H264Encoder
from picamera2.outputs import FfmpegOutput

THRESHOLD = 20
HOLD_TIME = 2.0
OUTPUT_PATH = ""
TRIG_COMMAND = ""

lsize = (320, 180)
picam2 = Picamera2()
video_config = picam2.create_video_configuration(main={"size": (1280, 720), "format": "RGB888"},
                                                 lores={"size": lsize, "format": "YUV420"})
picam2.configure(video_config)
encoder = H264Encoder(1000000)
picam2.start()

w, h = lsize
prev = None
encoding = False
ltime = 0

while True:
    cur = picam2.capture_buffer("lores")
    cur = cur[:w * h].reshape(h, w)
    if prev is not None:
        # Measure pixels differences between current and
        # previous frame
        mse = np.square(np.subtract(cur, prev)).mean()
        if mse > THRESHOLD:
            if not encoding:
                encoder.output = FfmpegOutput(f"{OUTPUT_PATH}/{int(time.time())}.mp4")

                picam2.start_encoder(encoder)
                encoding = True
                print("New Motion", mse)
                if TRIG_COMMAND != "":
                    subprocess.run([TRIG_COMMAND])
            ltime = time.time()
        else:
            if encoding and time.time() - ltime > HOLD_TIME:
                picam2.stop_encoder()
                encoding = False
    prev = cur
