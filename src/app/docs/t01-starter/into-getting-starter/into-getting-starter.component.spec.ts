import { ComponentFixture, TestBed } from '@angular/core/testing';

import { IntoGettingStarterComponent } from './into-getting-starter.component';

describe('IntoGettingStarterComponent', () => {
  let component: IntoGettingStarterComponent;
  let fixture: ComponentFixture<IntoGettingStarterComponent>;

  beforeEach(async () => {
    await TestBed.configureTestingModule({
      imports: [IntoGettingStarterComponent]
    })
    .compileComponents();

    fixture = TestBed.createComponent(IntoGettingStarterComponent);
    component = fixture.componentInstance;
    fixture.detectChanges();
  });

  it('should create', () => {
    expect(component).toBeTruthy();
  });
});
