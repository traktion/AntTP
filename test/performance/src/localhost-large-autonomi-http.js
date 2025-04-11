import http from 'k6/http';

export default function () {
  http.get('http://localhost:8080/91d16e58e9164bccd29a8fd8d25218a61d8253b51c26119791b2633ff4f6b309/to-autonomi.mp4');
  http.get('http://localhost:8080/cec7a9eb2c644b9a5de58bbcdf2e893db9f0b2acd7fc563fc849e19d1f6bd872/st-patrick-monument.mp4');
  http.get('http://localhost:8080/b6ec9f0f84cf6236dc42d3624679649f51024a57a58b2805552bb3aa690244dd/newcastle-promenade.mp4');
}
