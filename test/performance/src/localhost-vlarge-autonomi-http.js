import http from 'k6/http';

export default function () {
  http.get('http://localhost:18888/e7bb1b87c1f0e07cdb76ba5e82a425a8da712940c2d3553aa6791494e92aa54d/ubuntu-16.04.6-desktop-i386.iso', { timeout: '600s' });
}
