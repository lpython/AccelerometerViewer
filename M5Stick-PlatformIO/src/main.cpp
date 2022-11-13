#include <M5StickCPlus.h>

#if CONFIG_FREERTOS_UNICORE 
  static const BaseType_t app_cpu = 0;
#else 
  static const BaseType_t app_cpu = 1;
#endif

static const int led_pin = GPIO_NUM_10;

static QueueHandle_t accel_read_queue;  // element = int, sent form task A

static TimerHandle_t repeat_timer = NULL;

float accX = 0;
float accY = 0;
float accZ = 0;

float gyroX = 0;
float gyroY = 0;
float gyroZ = 0;

void toggleLED(void *parameter) {
  while(1) {
    digitalWrite(led_pin, HIGH);
    vTaskDelay(500 / portTICK_PERIOD_MS);
    digitalWrite(led_pin, LOW);
    vTaskDelay(500 / portTICK_PERIOD_MS);
  }
}

void setup() {
  M5.begin(false, true, true);
  M5.Imu.Init(); 

  vTaskDelay(1000 / portTICK_PERIOD_MS);

  Serial.println();
  Serial.println("---FreeRTOS Task Demo---");

  Serial.print("Setup and loop task running on core");
  Serial.print(xPortGetCoreID());
  Serial.print(" with priority ");
  Serial.println(uxTaskPriorityGet(NULL));

  pinMode(led_pin, OUTPUT);

  xTaskCreatePinnedToCore(
    toggleLED,
    "blinky",
    1024,
    NULL,
    10,
    NULL,
    app_cpu
  );

}

void loop() {
  M5.Imu.getGyroData(&gyroX,&gyroY,&gyroZ);
  M5.Imu.getAccelData(&accX,&accY,&accZ);

  
  Serial.printf("%.2f %.2f %.2f %.5f %.5f %.5f\n", gyroX, gyroY, gyroZ, accX, accY, accZ); 

  vTaskDelay(300);
}