# Performance Testing

## Usage

K6 is used for performance testing. Usage:

`cat src/localhost-autonomi-http.js; k6 run -u 10 -i 1000 results/2025-04-15/localhost-autonomi-http.js`

## Results

The localhost results are executed from a development laptop using a wifi connection on a 100 MB / 1 GB connection.
The connection performs ant node running duties, in addition to many other activities, but it is not congested.
So, these results are indicative of the performance of AntTP using the live Autonomi Network, rather than definitive.

Note that no browser based caching is used, but AntTP will cache the archive listings where applicable.