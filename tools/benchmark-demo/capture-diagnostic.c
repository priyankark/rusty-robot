// Independently exercise the same legacy macOS entry point used by pinned RobotJS.
// dlsym keeps this diagnostic buildable with SDKs marking the API unavailable.
#include <ApplicationServices/ApplicationServices.h>
#include <dlfcn.h>
#include <stdio.h>
int main(void) {
    typedef CGImageRef (*capture_fn)(CGDirectDisplayID, CGRect);
    capture_fn capture = (capture_fn)dlsym(RTLD_DEFAULT, "CGDisplayCreateImageForRect");
    printf("{\"permission\":%s,\"symbol_present\":%s,\"display\":%u,\"captures\":[", CGPreflightScreenCaptureAccess()?"true":"false",capture?"true":"false",CGMainDisplayID());
    if(!capture) return 1;
    int sizes[][2]={{1,1},{100,100},{1200,800}};
    for(int i=0;i<3;i++) {
        CGImageRef image=capture(CGMainDisplayID(),CGRectMake(360,140,sizes[i][0],sizes[i][1]));
        printf("%s{\"requested\":[%d,%d],\"returned_image\":%s}",i?",":"",sizes[i][0],sizes[i][1],image?"true":"false");
        if(image)CGImageRelease(image);
    }
    printf("]}\n");return 0;
}
