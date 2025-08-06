import http from 'k6/http';

export default function () {
  http.get('http://localhost:18888/62003e683b5a792f425a75c5d7d99d06e80f7047be8de8176b7d295e510b3b4c/wlp2gwHKFkZgtmSR3NB0oRJfbwhT.59829128118fe44e.woff2', { timeout: '600s' });
  http.get('http://localhost:18888/62003e683b5a792f425a75c5d7d99d06e80f7047be8de8176b7d295e510b3b4c/scripts.7c714e1d41c4921d.js', { timeout: '600s' });
  http.get('http://localhost:18888/cec7a9eb2c644b9a5de58bbcdf2e893db9f0b2acd7fc563fc849e19d1f6bd872/clean-green-immutable-dream.md', { timeout: '600s' });
  http.get('http://localhost:18888/62003e683b5a792f425a75c5d7d99d06e80f7047be8de8176b7d295e510b3b4c/runtime.74408d8ec77169a8.js', { timeout: '600s' });
  http.get('http://localhost:18888/62003e683b5a792f425a75c5d7d99d06e80f7047be8de8176b7d295e510b3b4c/xMQVuFNaVa6YuW0ZDK-y.b41e4968af1deb81.woff2', { timeout: '600s' });
  http.get('http://localhost:18888/62003e683b5a792f425a75c5d7d99d06e80f7047be8de8176b7d295e510b3b4c/polyfills.2e7f987c243a34c6.js', { timeout: '600s' });
}
