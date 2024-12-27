const canvas = document.getElementById("canvas") as HTMLCanvasElement;
const ctx = canvas.getContext("2d");

if (ctx) {
    ctx.fillStyle = "blue";
    ctx.fillRect(10, 10, 100, 100);
}
