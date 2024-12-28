import { defineConfig } from "vite";

export default defineConfig({
    build: {
        outDir: "../static/js",
        emptyOutDir: true,
        rollupOptions: {
            input: "src/index.ts",
            output: {
                entryFileNames: "index.js",
                format: "es",
            },
        },
    },
    server: {
        proxy: {
            "/events": "http://localhost:8000", // Adjust port if needed
        },
    },
});
