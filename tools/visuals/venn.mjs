import sharp from 'sharp';
import {fileURLToPath} from 'node:url';

export async function exportVenn() {
  const source = new URL('../../assets/visuals/source/venn.png', import.meta.url);
  const {data,info} = await sharp(fileURLToPath(source)).removeAlpha().raw().toBuffer({resolveWithObject:true});
  const {width,height} = info, pixels = width * height;
  const outside = new Uint8Array(pixels), queue = new Int32Array(pixels);
  let head = 0, tail = 0;
  // Only erase paper connected to the image border. The circle outlines stop
  // the flood, preserving their opaque fills, intersections and all lettering.
  function visit(i) {
    if(outside[i] || Math.min(data[i*3],data[i*3+1],data[i*3+2]) < 230) return;
    outside[i] = 1; queue[tail++] = i;
  }
  for(let x=0;x<width;x++) { visit(x); visit((height-1)*width+x); }
  for(let y=0;y<height;y++) { visit(y*width); visit(y*width+width-1); }
  while(head < tail) {
    const i=queue[head++], x=i%width;
    if(x) visit(i-1);
    if(x<width-1) visit(i+1);
    if(i>=width) visit(i-width);
    if(i<pixels-width) visit(i+width);
  }
  for(const dark of [false,true]) {
    const output = Buffer.alloc(pixels*4);
    for(let i=0;i<pixels;i++) {
      for(let c=0;c<3;c++) output[i*4+c] = dark ? 255-data[i*3+c] : data[i*3+c];
      output[i*4+3] = outside[i] ? 0 : 255;
    }
    const destination = new URL(`../../assets/vrs-venn${dark?'-dark':''}.png`, import.meta.url);
    await sharp(output,{raw:{width,height,channels:4}}).png().toFile(fileURLToPath(destination));
  }
}
