# pixelflut-client

Draws a bouncing image on a pixelflut server. Either regular raster images or svg images can be used. Additionally, a stroke can be drawn around the image to make it more visible.

```bash
Usage: pixelflut-client [OPTIONS] --host <HOST> --image-path <IMAGE_PATH>

Options:
  -H, --host <HOST>              
  -p, --port <PORT>              [default: 1337]
      --resize <RESIZE>          [default: 350]
      --drift-x <DRIFT_X>        [default: 12]
      --drift-y <DRIFT_Y>        [default: 9]
      --image-path <IMAGE_PATH>  
      --draw-rate <DRAW_RATE>    [default: 60]
      --stroke <STROKE>          [default: 4]
      --jitter                   
  -h, --help                     Print help
  -V, --version                  Print version
```