from typing import Protocol, Tuple, Any

import numpy as np


class Context(Protocol):
    def output(self) -> np.ndarray:
        """Returns a mutable numpy array representing the output image."""
        ...

    def max_output_size(self) -> Tuple[int, int]:
        """Returns the maximum allowable output (height, width) pair."""
        ...

    def set_output_size(self, height: int, width: int) -> None:
        """Sets the dimensions of the output image."""
        ...

    def register_image_input(self, name: str) -> None:
        """Specifies a layer input on the effect."""
        ...

    def register_int(self, name: str, min: int = -100, max: int = 100, default: int = 0) -> None:
        """Specifies an integer input which can be keyframed from After Effects."""
        ...

    def register_float(self, name: str, min: float = -100.0, max: float = 100.0, default: float = 0.0) -> None:
        """Specifies a float input which can be keyframed from After Effects."""
        ...

    def register_bool(self, name: str, default: bool = False) -> None:
        """Specifies a bool input."""
        ...

    def register_point(self, name: str, min: Tuple[float, float] = (-100.0, -100.0),
                       max: Tuple[float, float] = (100.0, 100.0),
                       default: Tuple[float, float] = (0.0, 0.0)) -> None:
        """Specifies a bounded 2d point input."""
        ...

    def register_color(self, name: str, default: Tuple[float, float, float, float] = (1.0, 1.0, 1.0, 1.0)) -> None:
        """Specifies a color input."""
        ...

    def get_input(self, name: str) -> Any:
        """Returns the input specified in setup under name with a value keyframed by the user."""
        ...

    def set_automatic_color_correction(self, on: bool) -> None:
        """Sets whether automatic color correction is enabled."""
        ...

    def set_sequential_mode(self, on: bool) -> None:
        """Sets whether the effect runs in sequential mode."""
        ...

    def is_sequential_mode(self) -> bool:
        """Returns True if the effect is running in sequential mode, False otherwise."""
        ...

    def time(self) -> float:
        """Returns the local comp time in seconds."""
        ...

    def build_info(self) -> str:
        """Returns a version string."""
        ...
