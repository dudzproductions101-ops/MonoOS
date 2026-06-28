CXX      := g++
CXXFLAGS := -std=c++17 -O2 -Wall -fPIC
LDFLAGS  := -shared

TARGET := libmonoos_camera.so
SRCS   := camera_pipeline.cpp
OBJS   := $(SRCS:.cpp=.o)

all: $(TARGET)
$(TARGET): $(OBJS)
	$(CXX) $(LDFLAGS) -o $@ $^
%.o: %.cpp
	$(CXX) $(CXXFLAGS) -c $< -o $@
clean:
	rm -f $(OBJS) $(TARGET)
