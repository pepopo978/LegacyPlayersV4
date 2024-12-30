// download-workbox.js - Run this script to download Workbox locally
const https = require('https');
const fs = require('fs');
const path = require('path');

const WORKBOX_VERSION = '5.1.2';
const DOWNLOAD_PATH = './public/js/workbox/';

// Create directory if it doesn't exist
if (!fs.existsSync(DOWNLOAD_PATH)) {
    fs.mkdirSync(DOWNLOAD_PATH, { recursive: true });
}

// List of Workbox files to download
const files = [
    'workbox-sw.js',
    'workbox-core.prod.js',
    'workbox-routing.prod.js',
    'workbox-strategies.prod.js',
    'workbox-precaching.prod.js'
];

files.forEach(file => {
    const url = `https://storage.googleapis.com/workbox-cdn/releases/${WORKBOX_VERSION}/${file}`;
    const filePath = path.join(DOWNLOAD_PATH, file);

    https.get(url, (response) => {
        const fileStream = fs.createWriteStream(filePath);
        response.pipe(fileStream);

        fileStream.on('finish', () => {
            console.log(`Downloaded ${file}`);
            fileStream.close();
        });
    }).on('error', (err) => {
        console.error(`Error downloading ${file}:`, err);
    });
});