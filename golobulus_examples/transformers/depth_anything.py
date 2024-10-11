import numpy as np
import torch
from PIL import Image
from transformers import pipeline

import golob_python.context

if torch.backends.mps.is_available():
    device = "mps"
elif torch.cuda.is_available():
    device = "cuda"
else:
    device = "cpu"

pipe = pipeline(
    task="depth-estimation",
    model="depth-anything/Depth-Anything-V2-Small-hf",
    device=device,
)


def setup(ctx: golob_python.context.Context):
    ctx.set_automatic_color_correction(False)
    ctx.register_image_input("image")
    pass


def run(ctx: golob_python.context.Context):
    input = ctx.get_input("image")

    if input is None:
        return

    ctx.set_output_size(input.shape[0], input.shape[1])
    output = ctx.output()

    im = Image.fromarray(input)
    depth = pipe(im)["depth"]
    arr = np.asarray(depth)
    output[..., :3] = arr[..., np.newaxis]
    output[..., 3] = 255
