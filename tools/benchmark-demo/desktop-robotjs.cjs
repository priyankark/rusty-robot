const robot=require(require('node:path').resolve(process.argv[2]));
const readline=require('node:readline');
readline.createInterface({input:process.stdin}).on('line',line=>{
 try{const v=JSON.parse(line);let result={};switch(v.op){
 case 'probe':result={accessibility:robot.getAccessibilityPermission(),capture:robot.getScreenCapturePermission(),displays:robot.getDisplays().length,cursor:robot.getMousePos()};break;
 case 'configure':robot.setMouseDelay(v.mouse_ms);robot.setKeyboardDelay(v.keyboard_ms);result={mouse_ms:v.mouse_ms,keyboard_ms:v.keyboard_ms};break;
 case 'click':robot.moveMouse(v.x,v.y);robot.mouseClick();break;
 case 'type':if(v.cpm)robot.typeStringDelayed(v.text,v.cpm);else robot.typeString(v.text);break;
 case 'key':robot.keyTap(v.key);break;
 case 'find':{const b=robot.screen.capture(v.x,v.y,v.width,v.height),p=b.findColor('00ff00',{tolerance:0.08});if(!p)throw Error('Target color missing from fresh capture');const q=b.toScreenPoint(p);result={x:q.x+8,y:q.y+8,width:b.width,height:b.height,scale:b.scaleX};break}
 default:throw Error('Unknown command')
 }console.log(JSON.stringify({ok:true,result}))}catch(e){console.log(JSON.stringify({ok:false,error:e.message}))}
});
