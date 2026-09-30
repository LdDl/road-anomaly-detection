# Zone editor

Static TypeScript and Fabric.js interface served by the detector's REST server. Run `npm ci && npm run build` to produce `dist`. `npm run check` performs TypeScript checks without building assets. Node.js 22 is used for the build and is not required on the deployment server.

Run `npm test` for MJPEG parser and connection lifecycle tests. They use simulated streams and image decoding without starting the detector.

The four-point drawing workflow and vertex controls are adapted. Whole stuff and controls were based on [`rust-road-traffic-ui`](https://github.com/LdDl/rust-road-traffic-ui). Canvas geometry uses original image coordinates and a viewport transform for display scaling.

MJPEG preview uses a frame reader adapted from the same UI (mentioned above) instead of a native multipart image. It reconnects after stream termination or six seconds without a decoded frame, with retries between 0.5 and 5 seconds. Only the newest waiting frame is retained while decoding, so a slow browser skips outdated preview frames. Hidden tabs stop the stream and reconnect when visible again. These changes affect only the preview; zone drafts remain in the editor.

The light theme follows the cream background, typography, outlined cards and offset shadows of [killmestats](https://github.com/andrew-pavlov-ua/killmestats/tree/master/client), using the [official Gleam color palette](https://gleam.run/branding/). This project is not an official Gleam product. Assets are served locally; no third-party scripts or fonts are loaded by the interface.

English is the default UI language. It has also Russian as a supported language. The choice is stored locally in the browser. UI translations live in `src/i18n.ts`, including status messages, confirmations and accessible labels.
