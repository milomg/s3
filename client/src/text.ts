import * as THREE from "three";

export function createText(text: string, height: number): THREE.Mesh {
  let scale = window.devicePixelRatio * 2;

  let canvas = document.createElement("canvas");
  let ctx = canvas.getContext("2d");

  ctx.font = height + "px Arial";
  let width = ctx.measureText(text).width;

  canvas.width = scale * width;
  canvas.height = scale * height;

  ctx.scale(scale, scale);
  ctx.font = height + "px Arial";
  ctx.textAlign = "center";
  ctx.textBaseline = "middle";
  ctx.fillStyle = "white";
  ctx.fillText(text, width / 2, height / 2);

  let texture = new THREE.CanvasTexture(canvas);

  let textPlane = new THREE.PlaneGeometry(width, height);
  let material = new THREE.MeshBasicMaterial({ map: texture, transparent: true, depthTest: false });
  let mesh = new THREE.Mesh(textPlane, material);

  return mesh;
}
