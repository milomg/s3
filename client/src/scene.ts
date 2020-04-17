import * as THREE from "three";

export let scene = new THREE.Scene();

{
  const near = 806;
  const far = 890;
  const color = 0x000008;
  scene.fog = new THREE.Fog(color, near, far);
  scene.background = new THREE.Color(color);
}

{
  let material = new THREE.LineBasicMaterial({ color: 0x0f0f0f });
  let geometry = new THREE.Geometry();
  geometry.vertices.push(new THREE.Vector3(0, 0, 0));
  geometry.vertices.push(new THREE.Vector3(0, 800, 0));
  geometry.vertices.push(new THREE.Vector3(800, 800, 0));
  geometry.vertices.push(new THREE.Vector3(800, 0, 0));
  geometry.vertices.push(new THREE.Vector3(0, 0, 0));
  let line = new THREE.Line(geometry, material);
  scene.add(line);
}

{
  let light = new THREE.AmbientLight(0x404040);
  scene.add(light);
}
{
  const light = new THREE.DirectionalLight(0xffffff - 0x404040, 1);
  light.position.set(100, 100, 100);
  scene.add(light);
}
