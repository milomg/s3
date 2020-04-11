import * as THREE from "three";
import { GLTFLoader } from "three/examples/jsm/loaders/GLTFLoader";
import { addGround } from "./ground";

const manager = new THREE.LoadingManager();
manager.onLoad = init;
export const models: { [key: string]: THREE.Object3D } = {
  spaceCraft1: undefined,
  spaceCraft3: undefined,
  spaceCraft6: undefined,
  crater: undefined,
  craterLarge: undefined,
  meteorFull: undefined,
  meteorFullRound: undefined,
  meteorHalf: undefined,
  rocks: undefined,
  rocksOre: undefined,
  rocksSmall: undefined,
  rocksSmallOre: undefined,
  rocksTall: undefined,
  rocksTallOre: undefined,
};
{
  const fbxLoader = new GLTFLoader(manager);
  for (const key of Object.keys(models)) {
    fbxLoader.load("gltf/" + key + ".glb", (gltf) => {
      models[key] = gltf.scene.children[0].children[0];
    });
  }
}

function init() {
  console.log("loaded");
  addGround();
}
