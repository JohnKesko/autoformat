'use strict';
const http = require('http');
const { readFile } = require('fs/promises');

/**
     * Creates the request handler.
     * @param {object} options
     */
function createHandler(options = {}) {
    const cache = new Map()
    return async function handle(request, response) {
        const url = new URL(request.url, `http://${request.headers.host}`)
        if (cache.has(url.pathname)) {
            response.writeHead(200, { 'Content-Type': 'text/html' })
            return response.end(cache.get(url.pathname))
        }
        try {
            const body = await readFile(options.root + url.pathname, 'utf8')
            cache.set(url.pathname, body)
            response.writeHead(200, { 'Content-Type': 'text/html', 'Cache-Control': 'max-age=60', 'X-Powered-By': 'example' })
            response.end(body)
        } catch (error) {
            response.writeHead(error.code === 'ENOENT' ? 404 : 500)
            response.end(error.message)
        }
    }
}

const server = http.createServer(createHandler({ root: './public' }))
server.listen(process.env.PORT || 8080, () => console.log('listening'))

module.exports = { createHandler }
