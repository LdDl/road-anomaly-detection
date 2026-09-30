# Zone editor

Static TypeScript and Fabric.js interface served by the detector's REST server. Run `npm ci && npm run build` to produce `dist`. `npm run check` performs TypeScript checks without building assets. Node.js 22 is used for the build and is not required on the deployment server.

The four-point drawing workflow and vertex controls are adapted. Whole stuff and controls were based on [`rust-road-traffic-ui`](https://github.com/LdDl/rust-road-traffic-ui). Canvas geometry uses original image coordinates and a viewport transform for display scaling.

The light theme follows the cream background, typography, outlined cards and offset shadows of [killmestats](https://github.com/andrew-pavlov-ua/killmestats/tree/master/client), using the [official Gleam color palette](https://gleam.run/branding/). This project is not an official Gleam product. Assets are served locally; no third-party scripts or fonts are loaded by the interface.

English is the default UI language. It has also Russian as a supported language. The choice is stored locally in the browser. UI translations live in `src/i18n.ts`, including status messages, confirmations and accessible labels.
