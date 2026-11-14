#!/usr/bin/env kotlin
plugins{
  kotlin("jvm") version "2.0.0"
    application
}
val ktorVersion:String by project
dependencies{
    implementation("io.ktor:ktor-server-core:$ktorVersion")
      testImplementation(kotlin("test"))
}
tasks.withType<Test>().configureEach{
    useJUnitPlatform()
  testLogging{events("passed","failed")}
}
application{mainClass.set("demo.MainKt")}
fun version(name:String)=providers.gradleProperty(name).get()
for(module in listOf("a","b"))
    include(module)
