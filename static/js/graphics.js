"use strict";
const canvas = document.getElementById("canvas");
const ctx = canvas.getContext("2d");
if (ctx) {
    ctx.fillStyle = "blue";
    ctx.fillRect(10, 10, 100, 100);
}
