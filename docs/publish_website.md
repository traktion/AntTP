# Publish Your Website

Publishing a website to the Autonomi Network is a straightforward process. Once published, your site is decentralized and permanently available.

## Recommended Resources

For a comprehensive guide and tools to help you publish, we recommend visiting **[PubAnt.com](https://pubant.com/)**. It provides an excellent resource for anyone looking to get their content onto the Autonomi Network.

## Quick Steps to Publish

1.  **Prepare your files:** Ensure all your website files are in a single directory. If it's an SPA, include an `app-config.json` for routing (see [Web Application Customisation](web_app.md)).
2.  **Upload to Autonomi:** Use the `ant` CLI or AntTP's upload features to push your directory as a public archive.
3.  **Obtain the XOR Address:** The upload process will provide you with a unique XOR address. This is the permanent address of your site on the network.
4.  **Access via AntTP:** You can now browse your site using `http://[XOR_ADDRESS]/` through any AntTP instance.

## Example: IMIM Blog

A great example of a project using AntTP is the **[IMIM (I am Immutable) blog](https://github.com/traktion/i-am-immutable-client)**. It allows authors to write in Markdown and publish directly to Autonomi.

IMIM demonstrates:
- How to use `routeMap` for Angular applications.
- How immutable file caching can provide near-zero latency.
- A practical application of decentralized web hosting.

You can view an example blog at:
`http://62003e683b5a792f425a75c5d7d99d06e80f7047be8de8176b7d295e510b3b4c/blog/705a5fa9b2b2ee9d1ec88f7f6cae45a9e40d4cf8ea202252c9d7e68eb6e17c8b#home`
*(Note: Requires AntTP proxy configuration)*

---
[<< Previous](web_app.md) | [Up](../README.md) | [Next >>](pnr.md)
