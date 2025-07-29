import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vite';

export default defineConfig({
	plugins: [sveltekit()],
	server: {
		port: 5173,
		proxy: {
			// Proxy API calls to the Rust backend
			'/dashboard/api': {
				target: 'http://localhost:3001',
				changeOrigin: true,
				secure: false
			},
			// Also proxy health check
			'/health': {
				target: 'http://localhost:3001',
				changeOrigin: true,
				secure: false
			}
		}
	}
});
