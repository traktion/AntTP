# Web Application Customisation

AntTP allows you to customise how your website or application behaves when hosted on the Autonomi Network. This is achieved through an `app-config.json` file.

## The `app-config.json` File

By including an `app-config.json` file in your archive, you can define routing rules for your application. This is particularly useful for Single Page Applications (SPAs) like Angular, React, or Vue, which require all requests to be routed through a single entry point (usually `index.html`).

### Example Configuration
```json
{
  "routeMap": {
    "": "index.html",
    "blog/*": "index.html",
    "blog/*/article/*": "index.html"
  }
}
```

### Key Mapping Rules
*   **Empty Key (`""`):** Maps the root URL of the archive to a specific file (e.g., serving `index.html` instead of a directory listing).
*   **Wildcards (`*`):** Allows you to map entire path patterns to a single file. For example, `blog/*` will serve `index.html` for any URL that starts with `blog/`.
*   **Custom Paths:** You can define any number of paths to suit your application's routing logic.

## Why Use a Route Map?

1.  **SPA Compatibility:** Modern frameworks handle routing internally within the browser. The server needs to serve the main HTML file for any route the user might land on or refresh.
2.  **Default Documents:** It provides a way to serve an index page by default, creating a more traditional web experience.
3.  **Clean URLs:** It enables prettier URLs without needing to explicitly include `.html` extensions or the full filename in every link.

## Deployment Process

1.  Create your `app-config.json` file in your project's root directory.
2.  Upload the directory as an archive to Autonomi.
3.  Access the archive address via AntTP and verify that the routing behaves as expected.

---
[<< Previous](archive.md) | [Up](../README.md) | [Next >>](publish_website.md)
