import tailwindcss from '@tailwindcss/vite';
import { defineConfig } from 'vite';

export default defineConfig({
    plugins: [tailwindcss()],
    build: {
        outDir: 'static',
        emptyOutDir: false,
        rollupOptions: {
            input: {
                main: 'frontend/main.ts',
            },
            output: {
                entryFileNames: '[name].js',
                assetFileNames: '[name].[ext]',
            },
        },
        minify: 'terser',
        terserOptions: {
            compress: { passes: 2 },
        },
    },
});
