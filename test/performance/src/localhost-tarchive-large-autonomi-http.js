import http from 'k6/http';

export default function () {
  http.get('http://localhost:18888/c9f7d82c093dcf47b3647a4a5b9ae64dfda038e5dabf3ff4f1f60ffaff870b5b/st-patrick-monument.mp4', { timeout: '600s' });
  http.get('http://localhost:18888/c9f7d82c093dcf47b3647a4a5b9ae64dfda038e5dabf3ff4f1f60ffaff870b5b/imim-mobile.mp4', { timeout: '600s' });
}
