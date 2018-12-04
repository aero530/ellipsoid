# Ellipsoid Pattern Generator

Generate an SVG flat pattern of a general ellipsoid.  Initially developed to generate flat patterns for laser cutting helmets from foam.

---

![screenshot_001](https://raw.githubusercontent.com/aero530/ellipsoid/master/screenshots/screenshot_001.PNG "screenshot")

![sample_output](https://github.com/aero530/ellipsoid/raw/master/screenshots/ellipsoid_a3.75in_b2.88in_c3.00in.png "sample_output")

![sample_output_spherical](https://github.com/aero530/ellipsoid/raw/master/screenshots/ellipsoid_a3.75in_b2.88in_c3.00in_spherical.png "sample_output_spherical")

---

## Features

* Specify arbitrary ellipsoid shape
* Open top or bottom of ellipsoid
* Add extra height to center, top, or bottom of ellipsoid
* Specify number of horizontal and vertical divisions
* Select units (mm, in, cm)
* Flatten ellipsoid spherically (from the top) or cylindrically (from the front)
* Preview flat pattern prior to saving file
* Visualize ellipsoid and flat pattern in 3D
* Output image includes dart lines for folding / glueing alignment
* Output image includes ruler to verify scale when printing
* Support for Inkscape layers

## Installation

Download the program installation file for the current release [here](https://github.com/aero530/ellipsoid/releases).

## Development Setup

This config works when using nodejs and yarn installed for windows (not through ubuntu in windows).

### Install / Update Node and yarn:

https://nodejs.org/en/

https://yarnpkg.com/en/

### Install shell launcher:

Add vs code extension shell launcher.

https://github.com/Tyriar/vscode-shell-launcher

Use it by crtl-shft-p 'shell'. Electron apps must be run from cmd.

### Clone the repo via git:

```cmd
git clone --depth=1 https://github.com/chentsulin/electron-react-boilerplate.git your-project-name
```

And then install dependencies with yarn (from the node.js command prompt).

```cmd
$ cd your-project-name
$ yarn
```

## Run

Start the app in the `dev` environment. This starts the renderer process in [**hot-module-replacement**](https://webpack.js.org/guides/hmr-react/) mode and starts a webpack dev server that sends hot updates to the renderer process:

```bash
$ yarn dev
```

Alternatively, you can run the renderer and main processes separately. This way, you can restart one process without waiting for the other. Run these two commands **simultaneously** in different console tabs:

```bash
$ yarn start-renderer-dev
$ yarn start-main-dev
```

If you don't need autofocus when your files was changed, then run `dev` with env `START_MINIMIZED=true`:

```bash
$ START_MINIMIZED=true yarn dev
```

## Packaging

To package apps for the local platform:

```bash
$ yarn package
```

To package apps for all platforms:

First, refer to [Multi Platform Build](https://www.electron.build/multi-platform-build) for dependencies.

Then,

```bash
$ yarn package-all
```

To package apps with options:

```bash
$ yarn package -- --[option]
```

:bulb: You can debug your production build with devtools by simply setting the `DEBUG_PROD` env variable:

```bash
DEBUG_PROD=true yarn package
```

## CSS Modules

This boilerplate is configured to use [css-modules](https://github.com/css-modules/css-modules) out of the box.

All `.css` file extensions will use css-modules unless it has `.global.css`.

If you need global styles, stylesheets with `.global.css` will not go through the
css-modules loader. e.g. `app.global.css`

If you want to import global css libraries (like `bootstrap`), you can just write the following code in `.global.css`:

```css
@import '~bootstrap/dist/css/bootstrap.css';
```

## SASS support

If you want to use Sass in your app, you only need to import `.sass` files instead of `.css` once:

```js
import './app.global.scss';
```

## Dispatching redux actions from main process

See [#118](https://github.com/electron-react-boilerplate/electron-react-boilerplate/issues/118) and [#108](https://github.com/electron-react-boilerplate/electron-react-boilerplate/issues/108)
