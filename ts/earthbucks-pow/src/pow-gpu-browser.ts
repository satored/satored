import * as tf from "@tensorflow/tfjs";
import GpuPow from "./pow-gpu";

type TF = typeof tf;

export class GpuPowBrowser extends GpuPow {
  tf: TF = tf;
}